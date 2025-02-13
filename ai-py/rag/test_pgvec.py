from langchain_postgres import PGVector

from repo_reader import AccumFileData

a = AccumFileData.default()


for g in a.good_files:
    print(g)


# vector_store = PGVector(
#     embeddings=embeddings,
#     collection_name="my_docs",
#     connection="postgresql+psycopg://...",
# )