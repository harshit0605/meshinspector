"""Service package.

Submodules are imported explicitly by route and worker code to avoid loading
legacy geometry engines on unrelated versioned SDK paths.
"""

__all__: list[str] = []
