from fastapi.testclient import TestClient
from main import app
from database import Base, engine, get_db
from sqlalchemy.orm import sessionmaker
from sqlalchemy import create_engine
from sqlalchemy.pool import StaticPool

# Setup in-memory SQLite for testing
SQLALCHEMY_DATABASE_URL = "sqlite:///:memory:"
engine = create_engine(
    SQLALCHEMY_DATABASE_URL, 
    connect_args={"check_same_thread": False},
    poolclass=StaticPool
)
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

Base.metadata.create_all(bind=engine)

def override_get_db():
    try:
        db = TestingSessionLocal()
        yield db
    finally:
        db.close()

app.dependency_overrides[get_db] = override_get_db

client = TestClient(app)

def test_root():
    response = client.get("/")
    assert response.status_code == 200
    assert response.json() == {"message": "Bing Crawling API is running (Prod Grade)"}

def test_trigger_crawl():
    response = client.post("/crawl", json={"keyword": "test travel"})
    assert response.status_code == 200
    assert "task_id" in response.json()
    assert response.json()["message"] == "Crawl started"

def test_get_crawl_status_not_found():
    response = client.get("/crawl/nonexistent-id")
    assert response.status_code == 404
    assert response.json()["detail"] == "Task not found"
