import os
import sys
import json
import argparse
import pandas as pd
from pathlib import Path

# Add current dir to path for compat
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kb_manager import add_file_to_kb, get_kb_dir, get_db

def index_comments(sec_uid: str, config: dict):
    """
    Index all comments of a user from SQLite to LanceDB.
    """
    from compat import get_data_dir
    db_path = get_data_dir() / "scraper_data" / sec_uid / "data" / sec_uid / "sqlite.db"
    
    if not db_path.exists():
        return {"error": f"数据库不存在: {db_path}"}
    
    import sqlite3
    conn = sqlite3.connect(str(db_path))
    query = "SELECT text, aweme_id FROM comments"
    df = pd.read_sql_query(query, conn)
    conn.close()
    
    if df.empty:
        return {"status": "success", "indexed": 0}
    
    # Save a temporary TXT for kb_manager to process
    temp_txt = get_kb_dir() / f"temp_comments_{sec_uid}.txt"
    with open(temp_txt, "w", encoding="utf-8") as f:
        for _, row in df.iterrows():
            f.write(f"视频ID[{row['aweme_id']}]: {row['text']}\n")
            
    res = add_file_to_kb(str(temp_txt), config)
    
    # Rename source in LanceDB to be more meaningful
    db = get_db()
    table = db.open_table("global_kb")
    table.update(f"source = '抖音评论_{sec_uid}'", where=f"source = '{temp_txt.name}'")
    
    # Clean up
    if temp_txt.exists():
        os.remove(temp_txt)
        
    return res

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--sec-uid", required=True)
    parser.add_argument("--config", required=True)
    
    args = parser.parse_args()
    config = json.loads(args.config)
    
    print(json.dumps(index_comments(args.sec_uid, config)))

if __name__ == "__main__":
    main()
