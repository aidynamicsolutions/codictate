"""
Pragmatic tests for MLX sidecar - focus on critical paths.

Run with: cd python-backend && uv run pytest tests/ -v
"""
import sys
import types

import pytest
from fastapi.testclient import TestClient
import server

app = server.app

client = TestClient(app)


class TestServerHealth:
    """Test 1: Server starts and responds"""
    
    def test_status_returns_running(self):
        """Critical: Server must respond to health checks"""
        response = client.get("/status")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "running"
        assert data["model_loaded"] == False
        assert data["model_path"] is None


class TestGenerateWithoutModel:
    """Test 2: Graceful handling when no model loaded"""
    
    def test_generate_rejects_without_model(self):
        """Critical: Don't crash, return clear error"""
        response = client.post("/generate", json={
            "prompt": "Translate: Hello world",
            "max_tokens": 50,
            "temperature": 0.7
        })
        assert response.status_code == 400
        assert "No model loaded" in response.json()["detail"]


class TestUnloadIdempotent:
    """Test 3: Unload works even when no model loaded"""
    
    def test_unload_when_empty(self):
        """Edge case: Unload should not crash"""
        response = client.post("/unload")
        assert response.status_code == 200
        data = response.json()
        assert data["model_loaded"] == False
        assert data["status"] == "no_model_loaded"


class TestLoadRequiresPath:
    """Test 4: Load validates input"""
    
    def test_load_requires_model_path(self):
        """Validation: model_path is required"""
        response = client.post("/load", json={})
        assert response.status_code == 422  # Validation error
    
    def test_load_returns_error_for_missing_path(self):
        """Validation: non-existent path returns 500"""
        response = client.post("/load", json={
            "model_path": "/nonexistent/path/to/model"
        })
        assert response.status_code == 500
        assert "Failed to load model" in response.json()["detail"]


class TestCleanModelResponse:
    """Test 5: Response cleaning handles edge cases correctly"""
    
    def test_removes_thinking_blocks(self):
        """Critical: <think> blocks should be stripped"""
        from server import clean_model_response
        
        raw = "<think>Let me analyze this...</think>Hello world"
        assert clean_model_response(raw) == "Hello world"
    
    def test_removes_multiline_thinking_blocks(self):
        """Edge case: multiline thinking blocks"""
        from server import clean_model_response
        
        raw = "<think>\nStep 1: Analyze\nStep 2: Process\n</think>The answer is 42"
        assert clean_model_response(raw) == "The answer is 42"
    
    def test_extracts_first_output_section(self):
        """Critical: Only first Output: section should be returned"""
        from server import clean_model_response
        
        raw = "Output: Hello\n\nOutput: World"
        assert clean_model_response(raw) == "Hello"
    
    def test_handles_repeated_text_pattern(self):
        """Edge case: Model looping on Text: should be cut off"""
        from server import clean_model_response
        
        raw = "Hello world Text: repeated Text: again"
        result = clean_model_response(raw)
        assert result == "Hello world"
    
    def test_handles_empty_string(self):
        """Edge case: Empty input returns empty output"""
        from server import clean_model_response
        
        assert clean_model_response("") == ""
        assert clean_model_response(None) == ""
    
    def test_strips_whitespace(self):
        """Basic: Whitespace should be stripped"""
        from server import clean_model_response
        
        raw = "  \n  Hello world  \n  "
        assert clean_model_response(raw) == "Hello world"
    
    def test_clean_text_passes_through(self):
        """Normal case: Clean text unchanged"""
        from server import clean_model_response
        
        raw = "This is a clean translation."
        assert clean_model_response(raw) == "This is a clean translation."


class TestChatTemplateFallback:
    """Regression: models without tokenizer.chat_template must still generate."""

    def test_generate_injects_qwen_template_when_chat_template_missing(self, monkeypatch):
        """Qwen3-4B-2507 tokenizer can omit chat_template metadata."""

        captured = {}

        class FakeTokenizer:
            eos_token = "<|im_end|>"
            additional_special_tokens = ["<|im_start|>", "<|im_end|>"]
            chat_template = None

            def apply_chat_template(self, messages, tokenize=False, add_generation_prompt=True, **kwargs):
                if not self.chat_template:
                    raise ValueError(
                        "Cannot use chat template functions because tokenizer.chat_template is not set and no template argument was passed!"
                    )
                prompt = f"<|im_start|>{messages[0]['role']}\n{messages[0]['content']}<|im_end|>\n"
                if add_generation_prompt:
                    prompt += "<|im_start|>assistant\n"
                return prompt

            def get_vocab(self):
                return {"<|im_start|>": 151644, "<|im_end|>": 151645}

            def encode(self, text):
                return [1, 2, 3]

        def fake_generate(_model, _tokenizer, prompt, **_kwargs):
            captured["prompt"] = prompt
            return "ok"

        sample_utils_mod = types.ModuleType("mlx_lm.sample_utils")
        sample_utils_mod.make_sampler = lambda **_kwargs: object()
        sample_utils_mod.make_logits_processors = lambda **_kwargs: object()

        mlx_lm_mod = types.ModuleType("mlx_lm")
        mlx_lm_mod.generate = fake_generate

        monkeypatch.setitem(sys.modules, "mlx_lm", mlx_lm_mod)
        monkeypatch.setitem(sys.modules, "mlx_lm.sample_utils", sample_utils_mod)
        monkeypatch.setattr(server, "model", object())
        monkeypatch.setattr(server, "tokenizer", FakeTokenizer())

        response = client.post(
            "/generate",
            json={
                "prompt": "Return JSON only",
                "max_tokens": 32,
                "temperature": 0.0,
                "top_p": 1.0,
                "min_p": 0.0,
                "system_ram_gb": 16,
            },
        )

        assert response.status_code == 200
        body = response.json()
        assert body["response"] == "ok"
        assert body["prompt_format_fallback"] is True
        assert "embedded_qwen_minimal" in body["prompt_format_mode"]
        assert captured["prompt"].startswith("<|im_start|>user\nReturn JSON only<|im_end|>\n")
        assert "<|im_start|>assistant\n" in captured["prompt"]
