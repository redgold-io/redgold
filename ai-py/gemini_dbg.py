import google.generativeai as genai
import os

genai.configure(api_key=os.environ["GEMINI_API_KEY"])