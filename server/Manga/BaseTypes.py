from pydantic import BaseModel, Field
from typing import Dict, List


class Comic(BaseModel):
    """
    Represents metadata about a comic.

    Attributes:
        id: Unique comic identifier.
        title: Dictionary mapping language codes to localized titles.
        cover_art: URL or file path to the comic’s cover image.
        availableLanguages: List of languages the comic is available in.
    """
    
    id: str = Field(..., description="Unique identifier for the comic")
    title: Dict[str, str] = Field(
        ...,
        description="Localized titles keyed by language code (e.g., 'en', 'jp')"
    )
    cover_art: str = Field(..., description="URL to the cover image of the comic")
    availableLanguages: List[str] = Field(
        ..., description="List of language codes in which the comic is available"
    )

    class Config:
        schema_extra = {
            "example": {
                "id": "7384342",
                "title": {
                    "en": "One Piece",
                    "jp": "ワンピース"
                },
                "cover_art": "https://example.com/covers/one-piece.jpg",
                "availableLanguages": ["en", "jp"]
            }
        }


class ChapterInfo(BaseModel):
    """
    Information about a single chapter.

    Attributes:
        id: Unique identifier for the chapter.
        chapter: Chapter number or serial as string.
    """
    
    id: str = Field(..., description="Unique identifier for the chapter")
    chapter: str = Field(..., description="Chapter number or serial as a string")


class VolumeData(BaseModel):
    """
    Represents a volume containing multiple chapters.

    Attributes:
        volume: Volume label or name, e.g. "Vol 1".
        chapters: Dictionary of chapters indexed by string keys.
    """
    
    volume: str = Field(..., description="Volume label, e.g. 'Vol 1'")
    chapters: Dict[str, ChapterInfo] = Field(
        ...,
        description="Dictionary of chapters within this volume, keyed by string indices"
    )


# Final type alias for the overall chapters dictionary
ChaptersDict = Dict[str, VolumeData]
