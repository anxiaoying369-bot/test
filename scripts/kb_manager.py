import os
import sys
import json
import argparse
import lancedb
import pandas as pd
from pathlib import Path
from openai import OpenAI
from pypdf import PdfReader

# Add current dir to path for compat
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from compat import get_data_dir

def get_kb_dir() -> Path:
    kb_dir = get_data_dir() / "knowledge_base"
    kb_dir.mkdir(parents=True, exist_ok=True)
    return kb_dir

def get_db():
    return lancedb.connect(str(get_kb_dir()))

def get_embedding(text: str, config: dict):
    # Use dedicated KB config, fallback to primary LLM config
    api_key = config['llm'].get('kb_api_key') or config['llm']['api_key']
    base_url = config['llm'].get('kb_base_url') or config['llm']['base_url']
    
    client = OpenAI(
        api_key=api_key,
        base_url=base_url
    )
    # Use model from config or fallback
    model = config['llm'].get('embedding_model') or "text-embedding-3-small"
    text = text.replace("\n", " ")
    return client.embeddings.create(input=[text], model=model).data[0].embedding

def parse_file(file_path: str) -> str:
    path = Path(file_path)
    if not path.exists():
        return ""
    
    ext = path.suffix.lower()
    if ext == ".txt":
        with open(path, "r", encoding="utf-8") as f:
            return f.read()
    elif ext == ".pdf":
        reader = PdfReader(path)
        text = ""
        for page in reader.pages:
            text += page.extract_text() + "\n"
        return text
    elif ext == ".json":
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
            return json.dumps(data, ensure_ascii=False)
    # Add more parsers if needed (docx, etc)
    return ""

def chunk_text(text: str, chunk_size: int = 500, overlap: int = 50):
    chunks = []
    start = 0
    while start < len(text):
        end = start + chunk_size
        chunks.append(text[start:end])
        start += chunk_size - overlap
    return chunks

def add_file_to_kb(file_path: str, config: dict):
    content = parse_file(file_path)
    if not content:
        return {"error": "无法解析文件内容或文件不存在"}
    
    chunks = chunk_text(content)
    db = get_db()
    
    table_name = "global_kb"
    
    data = []
    for i, chunk in enumerate(chunks):
        embedding = get_embedding(chunk, config)
        data.append({
            "vector": embedding,
            "text": chunk,
            "source": os.path.basename(file_path),
            "chunk_id": i
        })
    
    if table_name in db.table_names():
        table = db.open_table(table_name)
        table.add(data)
    else:
        db.create_table(table_name, data=data)
    
    return {"status": "success", "chunks_added": len(data)}

def search_kb(query: str, config: dict, limit: int = 5):
    db = get_db()
    table_name = "global_kb"
    
    if table_name not in db.table_names():
        return []
    
    query_vector = get_embedding(query, config)
    table = db.open_table(table_name)
    
    results = table.search(query_vector).limit(limit).to_list()
    
    # Remove vectors from results for JSON output
    for r in results:
        if "vector" in r:
            del r["vector"]
            
    return results

def list_files():
    db = get_db()
    table_name = "global_kb"
    if table_name not in db.table_names():
        return []
    
    table = db.open_table(table_name)
    df = table.to_pandas()
    if df.empty:
        return []
    
    files = df['source'].unique().tolist()
    return files

def delete_file(filename: str):
    db = get_db()
    table_name = "global_kb"
    if table_name not in db.table_names():
        return {"error": "知识库为空"}
    
    table = db.open_table(table_name)
    table.delete(f"source = '{filename}'")
    return {"status": "success"}

def get_file_details(filename: str):
    db = get_db()
    table_name = "global_kb"
    if table_name not in db.table_names():
        return []
    
    table = db.open_table(table_name)
    # Search for all records where source equals the filename
    results = table.to_pandas()
    if results.empty:
        return []
    
    file_chunks = results[results['source'] == filename].sort_values('chunk_id')
    
    # Convert to list of dicts and remove vector
    details = []
    for _, row in file_chunks.iterrows():
        details.append({
            "text": row['text'],
            "chunk_id": int(row['chunk_id']),
            "source": row['source']
        })
    return details

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["add", "search", "list", "delete", "details"])
    parser.add_argument("--file", help="File path to add")
    parser.add_argument("--query", help="Search query")
    parser.add_argument("--config", help="JSON config string")
    parser.add_argument("--filename", help="Filename to delete or get details")
    
    args = parser.parse_args()
    
    config = {}
    if args.config:
        config = json.loads(args.config)
    
    if args.action == "add":
        print(json.dumps(add_file_to_kb(args.file, config)))
    elif args.action == "search":
        print(json.dumps(search_kb(args.query, config)))
    elif args.action == "list":
        print(json.dumps(list_files()))
    elif args.action == "delete":
        print(json.dumps(delete_file(args.filename)))
    elif args.action == "details":
        print(json.dumps(get_file_details(args.filename)))

if __name__ == "__main__":
    main()
