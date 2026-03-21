"""Pydantic models for API request/response types."""

from pydantic import BaseModel, Field
from typing import Optional, List, Tuple
from enum import Enum


class MaterialType(str, Enum):
    """Supported material types with densities."""
    GOLD_24K = "gold_24k"
    GOLD_22K = "gold_22k"
    GOLD_18K = "gold_18k"
    GOLD_14K = "gold_14k"
    GOLD_10K = "gold_10k"
    SILVER_925 = "silver_925"
    PLATINUM = "platinum"


class UploadResponse(BaseModel):
    """Response from file upload endpoint."""
    model_id: str
    filename: str
    file_format: str
    preview_url: str


class AnalysisResult(BaseModel):
    """Mesh analysis results."""
    volume_mm3: float = Field(..., description="Volume in cubic millimeters")
    weight_g: float = Field(..., description="Weight in grams for selected material")
    bbox_mm: Tuple[float, float, float] = Field(..., description="Bounding box dimensions (x, y, z) in mm")
    is_watertight: bool = Field(..., description="Whether mesh is watertight/manifold")
    vertex_count: int
    face_count: int


class ProcessRequest(BaseModel):
    """Request for mesh processing."""
    model_id: str
    ring_size: Optional[float] = Field(None, ge=3, le=15, description="Target ring size (US standard)")
    wall_thickness_mm: Optional[float] = Field(None, ge=0.3, le=5.0, description="Wall thickness for hollowing")
    target_weight_g: Optional[float] = Field(None, ge=0.5, description="Target weight in grams")
    material: MaterialType = MaterialType.GOLD_18K


class ProcessResponse(BaseModel):
    """Response from processing endpoint."""
    model_id: str
    original_weight_g: float
    final_weight_g: float
    wall_thickness_mm: Optional[float]
    ring_size: Optional[float]
    preview_url: str
    download_url_glb: str
    download_url_stl: str
    # New fields for adaptive hollowing
    achieved_weight_g: Optional[float] = None
    iterations: Optional[int] = None
    warning: Optional[str] = None


class ErrorResponse(BaseModel):
    """Standard error response."""
    error: str
    detail: Optional[str] = None
