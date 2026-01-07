# /// script
# dependencies = [
#   "fastapi>=0.115.0",
#   "uvicorn>=0.32.0",
#   "mlx-lm>=0.22.0",
# ]
# ///
"""
MLX Local AI Server for Handy

A FastAPI server that provides local LLM inference using Apple's MLX framework
via the mlx-lm library (https://github.com/ml-explore/mlx-lm).

This server is spawned as a sidecar process by the Tauri backend and communicates
via HTTP on localhost:5000.

Usage:
    uv run python-backend/server.py
"""

import gc
import logging
from contextlib import asynccontextmanager
from typing import Optional

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import uvicorn

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Global model state
model = None
tokenizer = None
current_model_path: Optional[str] = None


class LoadRequest(BaseModel):
    """Request to load a model from a local path or HuggingFace repo."""
    model_path: str


class GenerateRequest(BaseModel):
    """Request to generate text from a prompt."""
    prompt: str
    max_tokens: int = 150  # Translation tasks need few tokens
    temperature: float = 0.7


class LoadResponse(BaseModel):
    """Response after loading a model."""
    status: str
    model_path: str


class GenerateResponse(BaseModel):
    """Response containing generated text."""
    response: str
    tokens_generated: int


class StatusResponse(BaseModel):
    """Server and model status."""
    status: str
    model_loaded: bool
    model_path: Optional[str] = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Handle startup and shutdown events."""
    logger.info("MLX Local AI Server starting up...")
    yield
    logger.info("MLX Local AI Server shutting down...")
    # Clean up model if loaded
    global model, tokenizer, current_model_path
    if model is not None:
        logger.info(f"Unloading model: {current_model_path}")
        model = None
        tokenizer = None
        current_model_path = None


app = FastAPI(
    title="MLX Local AI Server",
    description="Local LLM inference server for Handy using mlx-lm",
    version="1.0.0",
    lifespan=lifespan,
)


@app.get("/status", response_model=StatusResponse)
async def get_status():
    """Check server status and whether a model is loaded."""
    return StatusResponse(
        status="running",
        model_loaded=model is not None,
        model_path=current_model_path,
    )


@app.post("/load", response_model=LoadResponse)
async def load_model(request: LoadRequest):
    """
    Load a model from a local path or HuggingFace repository.
    
    The model_path can be:
    - A local directory containing the model files
    - A HuggingFace repository ID (e.g., "mlx-community/Qwen3-0.6B-4bit")
    """
    global model, tokenizer, current_model_path
    
    try:
        # Import here to avoid slow startup if server is just checking status
        from mlx_lm import load
        
        logger.info(f"Loading model from: {request.model_path}")
        
        # Unload existing model if any
        if model is not None:
            logger.info(f"Unloading previous model: {current_model_path}")
            model = None
            tokenizer = None
            gc.collect()
        
        # Load the new model
        model, tokenizer = load(request.model_path)
        current_model_path = request.model_path
        
        logger.info(f"Model loaded successfully: {request.model_path}")
        return LoadResponse(status="loaded", model_path=request.model_path)
        
    except Exception as e:
        logger.error(f"Failed to load model: {e}")
        raise HTTPException(status_code=500, detail=f"Failed to load model: {str(e)}")


@app.post("/generate", response_model=GenerateResponse)
async def generate_text(request: GenerateRequest):
    """
    Generate text from a prompt using the loaded model.
    
    Requires a model to be loaded first via /load endpoint.
    Uses Qwen3 chat template with thinking mode disabled for efficient inference.
    """
    global model, tokenizer
    
    if model is None or tokenizer is None:
        raise HTTPException(status_code=400, detail="No model loaded. Call /load first.")
    
    try:
        import re
        from mlx_lm import generate
        from mlx_lm.sample_utils import make_sampler
        
        logger.info(f"Generating text (max_tokens={request.max_tokens}, temp={request.temperature})")
        logger.info(f"=== INPUT PROMPT ===\n{request.prompt}\n=== END INPUT ===")
        
        # Apply Qwen3 chat template with thinking disabled for translation tasks
        # This formats the prompt properly with <|im_start|> and <|im_end|> tokens
        messages = [{"role": "user", "content": request.prompt}]
        try:
            formatted_prompt = tokenizer.apply_chat_template(
                messages,
                tokenize=False,
                add_generation_prompt=True,
                enable_thinking=False,  # Disable thinking for efficient translation
            )
            logger.debug(f"Applied chat template, formatted length: {len(formatted_prompt)}")
        except TypeError:
            # Fallback for tokenizers that don't support enable_thinking
            formatted_prompt = tokenizer.apply_chat_template(
                messages,
                tokenize=False,
                add_generation_prompt=True,
            )
            logger.debug("Using chat template without enable_thinking parameter")
        
        logger.info(f"=== FORMATTED PROMPT ===\n{formatted_prompt}\n=== END FORMATTED ===")
        
        # Create sampler with recommended settings for Qwen3 non-thinking mode
        sampler = make_sampler(
            temp=request.temperature,
            top_p=0.8,
            min_tokens_to_keep=1,
        )
        
        # Create logits processors for repetition penalty (separate from sampler)
        from mlx_lm.sample_utils import make_logits_processors
        logits_processors = make_logits_processors(
            repetition_penalty=1.15,  # Penalize repetition to avoid loops
            repetition_context_size=64,  # Look back 64 tokens for repetition
        )
        
        response = generate(
            model,
            tokenizer,
            prompt=formatted_prompt,
            max_tokens=request.max_tokens,
            sampler=sampler,
            logits_processors=logits_processors,  # Add repetition penalty
            verbose=False,
        )
        
        logger.info(f"=== RAW OUTPUT ===\n{response}\n=== END RAW OUTPUT ===")
        
        # Clean response: remove any thinking blocks and strip whitespace
        if response:
            # Remove <think>...</think> blocks if present
            response = re.sub(r'<think>.*?</think>', '', response, flags=re.DOTALL)
            
            # Remove repeated "Output:" sections - take only the first one
            if 'Output:' in response:
                parts = response.split('Output:')
                # Take the first non-empty part after "Output:"
                for part in parts[1:]:  # Skip first part before any "Output:"
                    clean_part = part.strip()
                    if clean_part:
                        response = clean_part.split('\n\n')[0]  # Take first paragraph
                        break
            
            # Remove repeated "Text:" patterns that indicate the model is looping
            if response.count('Text:') > 1:
                # Take only content before the first "Text:" repetition
                first_text_idx = response.find('Text:')
                if first_text_idx > 0:
                    response = response[:first_text_idx].strip()
                elif first_text_idx == 0:
                    # If starts with Text:, find next occurrence
                    second_text_idx = response.find('Text:', 5)
                    if second_text_idx > 0:
                        response = response[:second_text_idx].strip()
            
            # Strip leading/trailing whitespace and newlines
            response = response.strip()
        
        logger.info(f"=== CLEANED OUTPUT ===\n{response}\n=== END CLEANED OUTPUT ===")
        
        # Count tokens in response (approximate)
        tokens_generated = len(tokenizer.encode(response)) if response else 0
        
        logger.info(f"Generated {tokens_generated} tokens")
        return GenerateResponse(response=response, tokens_generated=tokens_generated)
        
    except Exception as e:
        logger.error(f"Generation failed: {e}")
        raise HTTPException(status_code=500, detail=f"Generation failed: {str(e)}")


@app.post("/unload", response_model=StatusResponse)
async def unload_model():
    """Unload the currently loaded model to free memory."""
    global model, tokenizer, current_model_path
    
    if model is None:
        return StatusResponse(
            status="no_model_loaded",
            model_loaded=False,
            model_path=None,
        )
    
    logger.info(f"Unloading model: {current_model_path}")
    model = None
    tokenizer = None
    current_model_path = None
    
    # Force garbage collection and clear Metal cache to free GPU memory
    gc.collect()
    try:
        import mlx.core as mx
        mx.metal.clear_cache()
    except Exception as e:
        logger.warning(f"Failed to clear MLX Metal cache: {e}")
    
    return StatusResponse(
        status="unloaded",
        model_loaded=False,
        model_path=None,
    )


if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="MLX Local AI Server for Handy")
    parser.add_argument("--port", type=int, default=5000, help="Port to run the server on")
    args = parser.parse_args()
    
    uvicorn.run(
        app,
        host="127.0.0.1",
        port=args.port,
        log_level="info",
    )
