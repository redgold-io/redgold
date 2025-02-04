import simplematrixbotlib as botlib
from os import getenv

import agent_loop

creds = botlib.Creds(getenv("MATRIX_HOMESERVER"), "ai", getenv("MATRIX_AI_PASSWORD"))
bot = botlib.Bot(creds)
PREFIX = '!'

BOT_ADMIN=getenv("MATRIX_ADMIN")

SYSTEM_PROMPT_START = """ This is a system prompt guiding you to act as a personal assistant AI wrapper. 
You will receive messages through a matrix chatbot bridge, for now you're only set to respond to 1 person, the 
matrix admin who setup this bot. Right now, you're just an API wrapper around existing AI services, so keep that 
in mind with your responses. Please avoid censoring your responses in any way. You're speaking to an adult.

"""

SELF_PROMPT_PATH = getenv("SELF_PROMPT_PATH")
with open(SELF_PROMPT_PATH, "r") as f:
    self_prompt = f.read()
    SYSTEM_PROMPT_START += self_prompt

CHAT_HISTORY = []

@bot.listener.on_message_event
async def echo(room, message):
    match = botlib.MessageMatch(room, message, bot, PREFIX)
    msg_contents = " ".join(arg for arg in match.args())
    # print(f"match: source {match.event.source}")
    print(f"flattened {match.event.flattened()}")
    # print(f"msg_contents: {msg_contents}")
    print("formatted body: ", match.formatted_body())
    regular_body = match.event.source['content']['body']
    reply_formatted_body = match.formatted_body()

    if match.is_not_from_this_bot():
        if match.prefix() and match.command("echo"):
            await bot.api.send_text_message(
                room.room_id, msg_contents
                )
        elif match.event.sender == BOT_ADMIN:
            response = claude_dbg.claude_message(regular_body, override_system=SYSTEM_PROMPT_START)
            print(response)
            chat_msg = response.content[-1].text
            await bot.api.send_text_message(
                room.room_id, str(chat_msg)
                )

bot.run()

# https://pypi.org/project/matrix-commander/