import backoff
import numpy as np
from elasticsearch import Elasticsearch
from elasticsearch.helpers import bulk, BulkIndexError

from repo_reader import AccumFileData


@backoff.on_exception(backoff.expo, Exception, max_tries=3)
def bulk_index_with_retry(es, docs):
    try:
        success, failed = bulk(es, docs)
        if failed:
            print(f"Failed documents:")
            for doc in failed:
                print(f"Document ID: {doc.get('_id')}")
                print(f"Error: {doc.get('error')}")
        return success, failed
    except BulkIndexError as e:
        print(f"Bulk indexing error: {str(e)}")
        print("Error details:")
        for error in e.errors:
            print(f"Document: {error}")
        raise


def search(query_text):
    # Connect to Elasticsearch
    es = Elasticsearch(["http://server:9202"])


    # Check Elasticsearch connection and version
    try:
        info = es.info()
        print(f"Connected to Elasticsearch {info['version']['number']}")
    except Exception as e:
        print(f"Error connecting to Elasticsearch: {e}")
        exit(1)

    # Define the index name and mapping
    index_name = "code_index"
    mapping = {
        "mappings": {
            "properties": {
                "filename": {"type": "text"},
                "content": {"type": "text"},
                "lines": {
                    "type": "nested",
                    "properties": {
                        "number": {"type": "integer"},
                        "text": {"type": "text"}
                    }
                },
                "code_vector": {"type": "dense_vector", "dims": 128}
            }
        }
    }

    # Create or update the index
    try:
        if es.indices.exists(index=index_name):
            es.indices.delete(index=index_name)
            print(f"Deleted existing index '{index_name}'")
        es.indices.create(index=index_name, body=mapping)
        print(f"Created index '{index_name}' with mapping")
    except Exception as e:
        print(f"Error creating index: {e}")
        exit(1)


    a = AccumFileData.default()
    files = a.indexed_files()

    docs = []

    for k in files:
        # print(k.path)
        lines = k.lines
        docs.append(
            {
                "_index": index_name,
                "_source": {
                    "filename": a.relative_path(k.path),
                    "content": k.contents,
                    "lines": [
                        {"number": i + 1, "text": line}
                        for i, line in enumerate(lines)
                    ],
                    "code_vector": np.random.rand(128).tolist()
                }
            }
        )

    # Index the documents

    try:
        success, failed = bulk_index_with_retry(es, docs)
        print(f"Indexed {success} documents")
        if failed:
            print(f"Failed to index {len(failed)} documents")
    except Exception as e:
        print(f"Fatal error during indexing: {e}")
        return []


    # Force a refresh to ensure the documents are searchable
    es.indices.refresh(index=index_name)

    def search_code(query_text, context_lines=2):
        query = {
            "query": {
                "bool": {
                    "must": [
                        {
                            "nested": {
                                "path": "lines",
                                "query": {
                                    "match": {
                                        "lines.text": query_text
                                    }
                                },
                                "inner_hits": {
                                    "highlight": {
                                        "fields": {
                                            "lines.text": {}
                                        }
                                    }
                                }
                            }
                        },
                        {
                            "script_score": {
                                "query": {"match_all": {}},
                                "script": {
                                    "source": "cosineSimilarity(params.query_vector, 'code_vector') + 1.0",
                                    "params": {"query_vector": np.random.rand(128).tolist()}
                                }
                            }
                        }
                    ]
                }
            }
        }

        results = es.search(index=index_name, body=query)

        llm_hits = []

        for hit in results['hits']['hits']:
            filename = hit['_source']['filename']
            # print(f"\nFile: {filename}")

            for inner_hit in hit['inner_hits']['lines']['hits']['hits']:
                line_number = inner_hit['_source']['number']
                highlighted_line = inner_hit['highlight']['lines.text'][0]

                # Get context lines
                start = max(1, line_number - context_lines)
                end = line_number + context_lines + 1
                context = hit['_source']['lines'][start - 1:end]

                llm_hit_text = ""
                llm_hit_text += f"Match at line {line_number} in file {filename}:\n"
                for ctx_line in context:
                    prefix = "  > " if ctx_line['number'] == line_number else "    "
                    llm_hit_text += f"{prefix}{ctx_line['number']}: {ctx_line['text']}"

                llm_hit_text += f"\nHighlighted: {highlighted_line}"
                llm_hits.append(llm_hit_text)
        return llm_hits
    return search_code(query_text)


def full_text_repo_search_tooldef():
    return {
        "name": "full_text_repo_search",
        "description": """Query elasticsearch index of Redgold repository for code or 
        document snippets. Includes surrounding context and line numbers""",
        "input_schema": {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": """The text to search for, for example, what a programmer 
                    might use Ctrl-Shift-F in IntelliJ to search, something like DataStoreContext::map_err_sqlx"""
                },
            },
            "required": ["query"]
        }
    }


def full_text_repo_search(input) -> list[str]:
    query = input['query']
    hits = search(query)
    return hits


def main():
    # Example usage
    search_query = "DataStoreContext::map_err_sqlx"
    print(f"\nSearching for: {search_query}")
    hits = search(search_query)
    print("\n\n".join(hits))


if __name__ == "__main__":
    main()
