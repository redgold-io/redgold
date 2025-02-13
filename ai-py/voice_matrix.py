# add load dotenv
import asyncio
from dotenv import load_dotenv
load_dotenv()

import simplematrixbotlib as botlib
from os import getenv

import agent_loop

# creds = botlib.Creds(getenv("MATRIX_HOMESERVER"), "ai", getenv("MATRIX_AI_PASSWORD"))
# bot = botlib.Bot(creds)
PREFIX = '!'

BOT_ADMIN=getenv("MATRIX_ADMIN")

from matrix_nio import AsyncClient


async def main():
    client = AsyncClient(getenv("MATRIX_HOMESERVER"), "ai")
    await client.login(getenv("MATRIX_AI_PASSWORD"))
    # await client.join(room_id)

if __name__ == "__main__":
    asyncio.run(main())