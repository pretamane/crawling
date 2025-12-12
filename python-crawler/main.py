from fastapi import FastAPI, BackgroundTasks, HTTPException, Depends
from pydantic import BaseModel
import uuid
import json
from sqlalchemy.orm import Session
from crawler import process_crawl_task
from database import init_db, get_db, Task

app = FastAPI()

# Initialize DB on startup
init_db()

class CrawlRequest(BaseModel):
    keyword: str

@app.post("/crawl")
async def trigger_crawl(request: CrawlRequest, background_tasks: BackgroundTasks, db: Session = Depends(get_db)):
    """
    Triggers a background crawl task for the given keyword.
    Returns a task_id to poll for results.
    """
    task_id = str(uuid.uuid4())
    
    # Create initial task record
    new_task = Task(id=task_id, keyword=request.keyword, status="processing")
    db.add(new_task)
    db.commit()
    
    background_tasks.add_task(process_crawl_task, task_id, request.keyword)
    return {"task_id": task_id, "message": "Crawl started"}

@app.get("/crawl/{task_id}")
async def get_crawl_status(task_id: str, db: Session = Depends(get_db)):
    """
    Returns the status and results of a crawl task.
    """
    task = db.query(Task).filter(Task.id == task_id).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")
    
    response = {
        "status": task.status,
        "keyword": task.keyword,
        "created_at": task.created_at,
        "results": json.loads(task.results_json) if task.results_json else [],
        "results": json.loads(task.results_json) if task.results_json else [],
        "first_page_html": task.first_page_html,
        "extracted_text": task.extracted_text,
        "meta_description": task.meta_description,
        "meta_author": task.meta_author,
        "meta_date": task.meta_date
    }
    return response

@app.get("/")
async def root():
    return {"message": "Bing Crawling API is running (Prod Grade)"}
