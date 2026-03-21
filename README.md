# MeshInspector V1

A web application for 3D jewelry manufacturability analysis. Resize rings, hollow for weight control, and export production-ready files.

## Quick Start

### Backend

```bash
cd meshinspector-backend

# Install dependencies with uv
uv sync

# Run development server
uv run uvicorn main:app --reload --port 8000
```

Backend runs at: http://localhost:8000

### Frontend

```bash
cd meshinspector-frontend

# Install dependencies
bun install

# Create environment file
echo "NEXT_PUBLIC_API_URL=http://localhost:8000" > .env.local

# Run development server
bun run dev
```

Frontend runs at: http://localhost:3000

## Features

- **Upload**: glTF, GLB, OBJ, STL files
- **Ring Sizing**: Resize to any standard US ring size (3-13)
- **Hollowing**: Set wall thickness for weight control
- **Analysis**: Volume, weight, dimensions, watertight check
- **Export**: Download GLB (preview) or STL (manufacturing)

## Tech Stack

- **Frontend**: Next.js 16, React Three Fiber, TanStack Query, Leva
- **Backend**: FastAPI, Trimesh, MeshLib
- **Processing**: STL as internal canonical format

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/upload` | Upload 3D model |
| GET | `/api/analyze/{id}` | Get mesh analysis |
| POST | `/api/process` | Resize + hollow |
| GET | `/api/preview/{id}` | Get GLB for viewer |
| GET | `/api/download/{id}/{format}` | Download GLB/STL |
