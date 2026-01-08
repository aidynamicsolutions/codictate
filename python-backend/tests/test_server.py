"""
Pragmatic tests for MLX sidecar - focus on critical paths.

Run with: cd python-backend && uv run pytest tests/ -v
"""
import pytest
from fastapi.testclient import TestClient
from server import app

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
