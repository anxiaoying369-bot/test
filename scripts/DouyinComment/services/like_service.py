import asyncio
from datetime import datetime
from typing import List, Dict
from tqdm import tqdm

from services.base_service import BaseService
from services.video_service import VideoService
from core.logger import logger
from core.api import CookieExpiredError
from utils.helpers import safe_str, safe_int, sleep_jitter


class LikeService(VideoService):
    data_type = "like"
    table_name = "likes"
    csv_filename = "likes.csv"
    
    async def fetch(self, page_size: int = 20, delay: float = 1.0,
                    limit: int = 0, **kwargs) -> List[Dict]:
        all_likes = []
        max_cursor = 0
        has_more = True
        
        pbar = tqdm(desc="采集喜欢", unit="条")
        
        try:
            while has_more:
                if limit > 0 and len(all_likes) >= limit:
                    break
                
                try:
                    response = await self.api.fetch_likes(self.sec_uid, max_cursor, page_size)
                    aweme_list = response.get("aweme_list", [])
                    has_more = response.get("has_more", False)
                    max_cursor = response.get("max_cursor", 0)
                    
                    if aweme_list:
                        all_likes.extend(aweme_list)
                        pbar.update(len(aweme_list))
                    else:
                        break
                        
                    if has_more:
                        await sleep_jitter(delay)
                        
                except CookieExpiredError:
                    raise
                except Exception as e:
                    logger.error(f"[采集] 获取喜欢失败: {e}")
                    break
        finally:
            pbar.close()
            
        return all_likes

    async def run(self, delay: float = 1.0, limit: int = 0, **kwargs) -> Dict:
        logger.info(f"[采集] 开始采集用户喜欢的视频: {self.sec_uid}")
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
