import asyncio
from datetime import datetime
from typing import List, Dict
from tqdm import tqdm

from services.base_service import BaseService
from core.logger import logger
from core.api import CookieExpiredError
from utils.helpers import safe_str, safe_int, sleep_jitter


class FollowerService(BaseService):
    data_type = "follower"
    id_field = "uid"
    table_name = "followers"
    csv_filename = "followers.csv"
    
    async def fetch(self, page_size: int = 20, delay: float = 1.0,
                    limit: int = 0, **kwargs) -> List[Dict]:
        all_followers = []
        cursor = 0
        has_more = True
        
        pbar = tqdm(desc="采集粉丝", unit="人")
        
        try:
            while has_more:
                if limit > 0 and len(all_followers) >= limit:
                    break
                
                try:
                    response = await self.api.fetch_followers(self.sec_uid, cursor, page_size)
                    followers = response.get("followers", [])
                    has_more = response.get("has_more", False)
                    cursor = response.get("cursor", 0)
                    
                    if followers:
                        all_followers.extend(followers)
                        pbar.update(len(followers))
                    else:
                        break
                        
                    if has_more:
                        await sleep_jitter(delay)
                        
                except CookieExpiredError:
                    raise
                except Exception as e:
                    logger.error(f"[采集] 获取粉丝失败: {e}")
                    break
        finally:
            pbar.close()
            
        return all_followers

    def process(self, raw_followers: List[Dict], **kwargs) -> List[Dict]:
        processed = []
        for f in raw_followers:
            processed.append({
                "uid": f.get("uid"),
                "short_id": f.get("short_id"),
                "nickname": safe_str(f.get("nickname")),
                "signature": safe_str(f.get("signature")),
                "avatar_url": f.get("avatar_thumb", {}).get("url_list", [None])[0],
                "sec_uid": f.get("sec_uid"),
            })
        return processed

    async def run(self, delay: float = 1.0, limit: int = 0, **kwargs) -> Dict:
        logger.info(f"[采集] 开始采集用户粉丝: {self.sec_uid}")
        await self.api.verify_cookie()
        
        start_time = datetime.now()
        raw = await self.fetch(delay=delay, limit=limit)
        processed = self.process(raw)
        
        save_result = self.storage.save(processed)
        
        duration = (datetime.now() - start_time).total_seconds()
        return {
            'total': len(processed),
            'new': save_result['csv'],
            'duration': f"{duration:.1f}秒"
        }
