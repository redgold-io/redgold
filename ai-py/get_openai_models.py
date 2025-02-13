from openai import OpenAI

client = OpenAI()
models = client.models.list()
print(models)


for model in models:
   print(model)

