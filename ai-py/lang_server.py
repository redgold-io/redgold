from multilspy import SyncLanguageServer
from multilspy.multilspy_config import MultilspyConfig
from multilspy.multilspy_logger import MultilspyLogger

from repo_reader import WORKSPACE_PATH



def run_lang_server():
    config = MultilspyConfig.from_dict({"code_language": "rust"}) # Also supports "python", "rust", "csharp", "typescript", "javascript", "go"
    logger = MultilspyLogger()
    lsp = SyncLanguageServer.create(config, logger, str(WORKSPACE_PATH))
    with lsp.start_server():
        result = lsp.request_definition(
            "relative/path/to/code_file.java", # Filename of location where request is being made
            163, # line number of symbol for which request is being made
            4 # column number of symbol for which request is being made
        )
        result2 = lsp.request_completions(
            ...
        )
        result3 = lsp.request_references(
            ...
        )
        result4 = lsp.request_document_symbols(
            ...
        )
        result5 = lsp.request_hover(
            ...
        )
        ...

# from monitors4codegen.multilspy import SyncLanguageServer
# from monitors4codegen.multilspy.multilspy_config import MultilspyConfig
# from monitors4codegen.multilspy.multilspy_logger import MultilspyLogger
# from pathlib import Path

# config = MultilspyConfig.from_dict({"code_language": "rust"})  # Also supports "python", "rust", "csharp"
# logger = MultilspyLogger()
# parent = Path.cwd().parent
# print("repo path", parent)
# lsp = SyncLanguageServer.create(config, logger, str(parent))
# with lsp.start_server():
#     result = lsp.request_definition(
#         "src/core/relay.rs", # Filename of location where request is being made
#         183, # line number of symbol for which request is being made
#         22 # column number of symbol for which request is being made
#     )
#     print(result)
#     # result2 = lsp.request_completions(
#     #
#     # )
#     # result3 = lsp.request_references(
#     #     ...
#     # )
#     result4 = lsp.request_document_symbols(
#         ...
#     )
#     # result5 = lsp.request_hover(
#     #     ...
#     # )
#     # ...