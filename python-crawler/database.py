from sqlalchemy import create_engine, Column, String, Integer, Text, DateTime
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import sessionmaker
from datetime import datetime
from config import settings

engine = create_engine(settings.DATABASE_URL, connect_args={"check_same_thread": False})
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

Base = declarative_base()

class Task(Base):
    __tablename__ = "tasks"

    id = Column(String, primary_key=True, index=True)
    keyword = Column(String, index=True)
    status = Column(String, default="processing")
    created_at = Column(DateTime, default=datetime.utcnow)
    
    # Storing results as JSON string for simplicity in SQLite
    results_json = Column(Text, nullable=True)
    first_page_html = Column(Text, nullable=True)
    
    # New fields for high-quality extraction
    extracted_text = Column(Text, nullable=True)
    meta_description = Column(Text, nullable=True)
    meta_author = Column(String, nullable=True)
    meta_date = Column(String, nullable=True)

def init_db():
    Base.metadata.create_all(bind=engine)

def get_db():
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()
