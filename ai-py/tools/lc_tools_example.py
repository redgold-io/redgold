# from langchain.tools import BaseTool
#
#
# class MyTool(BaseTool):
#     name = "my_tool"
#     description = "Does something cool"
#
#     def _run(self, query: str):
#         return f"Result for {query}"
#
#
# tool = MyTool()
# openai_function = tool.to_openai_function()
# # This gives you the JSON spec OpenAI expects
