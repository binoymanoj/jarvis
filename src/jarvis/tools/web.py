"""Web automation and query tools for Jarvis."""

import asyncio
import re
import shutil
import urllib.parse
import urllib.request
from typing import Optional, Tuple
from jarvis.core.logger import log


class WebNavigator:
    """Handles web browsing, searches, and direct YouTube video resolution."""

    def __init__(self):
        self._browser_bin = shutil.which("omarchy-launch-browser") or shutil.which("xdg-open") or "xdg-open"

    async def open_url(self, url: str) -> str:
        """Open a specific URL in the user's default browser."""
        url = url.strip()
        if not url.startswith(("http://", "https://")):
            url = f"https://{url}"

        log.info(f"[cyan]Opening URL in browser: {url}[/cyan]")
        try:
            proc = await asyncio.create_subprocess_exec(
                self._browser_bin,
                url,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
                start_new_session=True,
            )
            try:
                await asyncio.wait_for(proc.wait(), timeout=0.15)
            except asyncio.TimeoutError:
                pass
            return f"Opened {url} in browser."
        except Exception as e:
            log.error(f"Failed to open URL '{url}': {e}")
            return f"Failed to open {url}: {e}"

    async def search_web(self, query: str, engine: str = "google") -> str:
        """Search the web using Google or DuckDuckGo."""
        query = query.strip()
        encoded = urllib.parse.quote(query)
        if engine.lower() == "duckduckgo":
            url = f"https://duckduckgo.com/?q={encoded}"
        else:
            url = f"https://www.google.com/search?q={encoded}"
        return await self.open_url(url)

    async def open_youtube(self, query: str) -> str:
        """Search YouTube for a video/topic and immediately open and play the top result."""
        query = query.strip()
        # If the user passed an actual YouTube URL directly
        if "youtube.com" in query or "youtu.be" in query:
            await self.open_url(query)
            return "Opened YouTube link."

        # Fetch top video ID and title asynchronously with fast fallback
        loop = asyncio.get_running_loop()
        try:
            video_info = await asyncio.wait_for(
                loop.run_in_executor(None, self._resolve_youtube_video, query),
                timeout=1.4,
            )
        except asyncio.TimeoutError:
            video_info = None

        if video_info:
            video_id, title = video_info
            watch_url = f"https://www.youtube.com/watch?v={video_id}"
            await self.open_url(watch_url)
            return f"Playing '{title}' on YouTube."
        else:
            # Fallback to YouTube search results page
            encoded = urllib.parse.quote(query)
            search_url = f"https://www.youtube.com/results?search_query={encoded}"
            await self.open_url(search_url)
            return f"Opened YouTube search for '{query}'."

    @staticmethod
    def _resolve_youtube_video(query: str) -> Optional[Tuple[str, str]]:
        """Scrapes the top video result ID and title from YouTube search HTML without external dependencies."""
        try:
            encoded = urllib.parse.quote(query)
            url = f"https://www.youtube.com/results?search_query={encoded}"
            req = urllib.request.Request(
                url,
                headers={"User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0"},
            )
            with urllib.request.urlopen(req, timeout=1.2) as resp:
                html = resp.read().decode("utf-8", errors="ignore")

            # Try to match videoId and title together
            match = re.search(
                r'\"videoRenderer\":\{\"videoId\":\"([a-zA-Z0-9_-]{11})\".*?\"title\":\{\"runs\":\[\{\"text\":\"(.*?)\"\}\]',
                html,
            )
            if match:
                video_id = match.group(1)
                title = match.group(2)
                return video_id, title

            # Fallback: match any videoId
            ids = re.findall(r'\"videoId\":\"([a-zA-Z0-9_-]{11})\"', html)
            if ids:
                return ids[0], query

            return None
        except Exception as e:
            log.warning(f"YouTube resolution failed: {e}")
            return None
