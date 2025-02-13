from dataclasses import dataclass
from pathlib import Path
import subprocess
from typing import Annotated, Optional
import os
import git
from github import Github

from langchain_core.tools import tool

from es_search import full_text_repo_search, search
from file_ux.create import create_file
from file_ux.file_viewer import read_file
from file_ux.git_diffs import get_git_diff
from git_scrape import redgold_issue_by_number, redgold_issues_brief
from tools.commands import ai_working_dir, redgold_cargo_rust_compile
from langchain_core.tools import InjectedToolArg


@dataclass
class ToolInjections:
    working_directory: Path = Path.home() / "ai" / "redgold"


@tool
def active_workspace_redgold_cargo_rust_compile(
    inject: Annotated[ToolInjections, InjectedToolArg]
) -> Annotated[str, "Error messages if any, or compiler output, otherwise list with success messages"]:
    """
    Compile the current Redgold Rust project stored in typical working data defaults to ~/ai/redgold.
    :return: List of error messages if any, otherwise list with success messages.
    """
    
    cargo_dir = inject.working_directory
    original_dir = os.getcwd()

    try:
        os.chdir(cargo_dir)

        # rustflags disable warnings
        env = os.environ.copy()
        env['RUSTFLAGS'] = '-A warnings'
        result = subprocess.run(['cargo', 'check'],
                                capture_output=True,
                                text=True,
                                env=env)

        if result.returncode == 0:
            return [f"Cargo check completed successfully in {cargo_dir}"]
        else:
            output = result.stderr
            lines = output.split("\n")
            output = []
            for i, line in enumerate(lines):
                stripped = line.strip()
                if stripped.startswith("Checking") or stripped.startswith("Compiling") or stripped.startswith("warning"):
                    continue
                output.append(line)
            return "\n".join(output)
    finally:
        os.chdir(original_dir)



def create_file_helper(
        working_dir: Path, 
        relative_path: str, 
        content: str,
        include_as_rust_pub_mod: bool = True
):
    rel = working_dir / relative_path
    parent_dir = rel.parent
    parent_dir.mkdir(parents=True, exist_ok=True)
    rel.write_text(content)
    if include_as_rust_pub_mod and rel.name.endswith(".rs"):
        # first check if libs.rs exists in the same parent directory
        # if not, check for mod.rs
        rel_p = Path(rel)
        fnm = rel_p.name
        cur = rel_p.parent
        lib = cur / "lib.rs"
        mod = cur / "mod.rs"
        export_str = f"\npub mod {fnm.split('.')[0]};\n"
        if lib.exists():
            lib.write_text(export_str, append=True)
        elif mod.exists():
            mod.write_text(export_str, append=True)
        else:
            return "No lib.rs or mod.rs found in the parent directory, not adding the export line"
    return "Success created file with content"
        

@tool
def add_text_to_top_or_bottom_of_file(
    text: Annotated[str, "The text to add to the top of the file, please include newlines if required including at end of text"],
    relative_path: Annotated[str, "The relative path of the file to add the text to"],
    inject: Annotated[ToolInjections, InjectedToolArg],
    top: Annotated[bool, "Whether to add the text to the top or bottom of the file"]
) -> Annotated[str, "Success or error message"]:
    """
    Add text to the top of a file. Useful to avoid having to edit particular lines. Will auto-create file if it doesn't exist.
    """
    
    cargo_dir = inject.working_directory
    rel = cargo_dir / relative_path
    if not rel.exists():
        create_file_helper(cargo_dir, relative_path, text)
    else:
        existing = rel.read_text()
        if top:
            new = text + "\n" + existing
        else:
            new = existing + "\n" + text
        rel.write_text(new)
    return "Success"


@tool
def get_git_diff_relative_to_dev(
    inject: Annotated[ToolInjections, InjectedToolArg]
) -> Annotated[str, "The git diff of the current repository relative to the dev branch"]:
    """
    Get the git diff of the current repository relative to the dev branch.
    """
    os.chdir(inject.working_directory)
    return subprocess.check_output(['git', 'diff', 'dev', '--unified=0'], text=True).strip()

injectable_tools = [
    "active_workspace_redgold_cargo_rust_compile",
    "add_text_to_top_or_bottom_of_file",
    "get_git_diff_relative_to_dev",
    "edit_active_workspace_file"
]


@tool
def index_and_full_text_repo_search(
    query: Annotated[str, "Text to search for"],
    num_results: Annotated[Optional[int], "The number of results to return. If None, defaults to 10"] = 10,
    context_lines_returned: Annotated[Optional[int], "The number of lines of context to return. If None, defaults to 5"] = 5
) -> Annotated[list[str], "List of results"]:
    """
    Search the entire Redgold repository for the given query. Rebuilds an update to date
    index each time
    :param query: The query to search for.
    :return: A list of results.
    """
    hits = search(query, num_results, context_lines_returned)
    if not hits:
        hits = ["No results found"]
    return hits


@tool
def edit_active_workspace_file(
    inject: Annotated[ToolInjections, InjectedToolArg],
    filename: Annotated[str, "Relative path"],
    starting_line: Annotated[Optional[int], "The (inclusive) 1-indexed starting line number to begin replacing. If None, starts at beginning"],
    ending_line: Annotated[Optional[int], "The (inclusive) 1-indexed ending line number to stop replacing. If None, goes to end of file"],
    replacement_text: Annotated[str, "The new text to insert. Please include \n newlines for it to work correctly"] = "",
) -> Annotated[str, "Status message indicating success or error"]:
    """
    Edit a file in the current workspace by replacing lines between starting_line and ending_line with replacement_lines.
    If the file doesn't exist, it will be created along with any necessary parent directories. 
    If lines exceed bounds, it will overwrite / remove them.
    Line numbers are 1-indexed and inclusive.
    """
    if replacement_text is None:
        replacement_text = ""

    rel = inject.working_directory / filename

    print(f"Editing file {rel}")
    # check if the file exists, if its parents do not exist, create them as directories
    if not os.path.exists(rel):
        create_file_helper(inject.working_directory, filename, replacement_text)
        return "Successfully created file and wrote to it"

    lines = rel.read_text().splitlines()

    if starting_line is None:
        start = 0
    elif starting_line < 1:
        start = 0
    else: 
        start = starting_line - 1
    
    if ending_line is None:
        end = len(lines)
    elif ending_line < 1:
        end = 0
    else:
        end = ending_line
    
    if ending_line >= len(lines):
        end = end

    if start >= len(lines):
        return "\n".join(lines) + "\n" + replacement_text
    
    before = lines[:start]
    after = lines[end:]

    # Split replacement text into lines to properly handle multi-line replacements
    replacement_lines = replacement_text.splitlines()
    new_lines = before + replacement_lines + after
    
    # Write the modified content back to the file
    rel.write_text('\n'.join(new_lines) + '\n')

    return f"Successfully edited file {filename}"



@tool
def active_workspace_read_file(
    filename: Annotated[str, "The relative path of the file to read within the current repository"],
    starting_line: Annotated[Optional[int], "The (inclusive) 1-indexed starting line number to begin reading. If None, starts at beginning"],
    ending_line: Annotated[Optional[int], "The (inclusive) 1-indexed ending line number to stop reading. If None, goes to end of file"]
) -> Annotated[str, "The content of the file, with line numbers added"]:
    """
    Read a file in the current workspace by returning the lines between starting_line and ending_line.
    Line numbers are 1-indexed and inclusive.
    """
    prefix = str(ai_working_dir())
    if not filename.startswith("/"):
        prefix += "/"
    filename = prefix + filename
    print(f"Reading file: {filename} from {prefix} with starting_line {starting_line} and ending_line {ending_line}")
    with open(filename, "r") as f:
        lines = f.readlines()
        processed_lines = []
        for i, line in enumerate(lines):
            line = f"{i+1}: {line}"
            if starting_line is not None and i < starting_line:
                continue
            if ending_line is not None and i > ending_line:
                break
            processed_lines.append(line)
    print(f"Finished reading lines {len(processed_lines)}")    
    return "\n".join(processed_lines)


# @tool
# def active_workspace_create_file(
#     repository_relative_path: Annotated[str, "The relative path of the file to create within the current repository"],
#     file_text_to_insert_to_new_file: Annotated[str, "The content of the file"],
#     include_as_rust_pub_mod: Annotated[bool, "Whether to include the file as a Rust pub mod"] = True
# ) -> Annotated[str, "Status message indicating success or error"]:
#     """
#     Create a new file in the current workspace with the given content.
#     If the file already exists, it will be overwritten.
#     """
#     rel_start = Path.home().joinpath("ai/redgold")
#     rel: str = repository_relative_path
#     rel = str(rel_start.joinpath(rel))
#     # Get parent directory path and create all necessary directories
#     parent_dir = Path(rel).parent
#     parent_dir.mkdir(parents=True, exist_ok=True)

#     with open(rel, "w") as f:
#         f.write(file_text_to_insert_to_new_file)
#         print(f"File created at {rel}")
#     if include_as_rust_pub_mod:
#         # first check if libs.rs exists in the same parent directory
#         # if not, check for mod.rs
#         rel_p = Path(rel)
#         fnm = rel_p.name
#         cur = rel_p.parent
#         lib = cur.joinpath("lib.rs")
#         mod = cur.joinpath("mod.rs")
#         export_str = f"\npub mod {fnm.split('.')[0]};\n"
#         if lib.exists():
#             with open(lib, "a") as f:
#                 f.write(export_str)
#         elif mod.exists():
#             with open(mod, "a") as f:
#                 f.write(export_str)
#         else:
#             print("No lib.rs or mod.rs found in the parent directory, not adding the export line")
#     return "Success"
        


@tool
def active_workspace_get_git_diff() -> Annotated[str, "The git diff of the current repository"]:
    """
    Get the git diff of the current repository.
    """
    return get_git_diff()[1]()


@tool
def get_git_issues() -> Annotated[list[str], "Titles and issue numbers and labels of issues in the current repository"]:
    """
    Get the git issues of the current repository.
    """
    return redgold_issues_brief()

@tool
def get_git_issue_by_number(number: Annotated[int, "The number of the issue to get"]
                            ) -> Annotated[str, "The issue"]:
    """
    Get the git issue by number.
    """
    return redgold_issue_by_number(number)

WORKSPACE_PATH = Path.home().joinpath("ai/redgold")

@tool
def create_fresh_dev_branch(
    branch_name: Annotated[str, "The name of the branch to create"]
) -> Annotated[str, "The status message indicating success or error"]:
    """
    Overwrite the existing workspace starting from a fresh dev branch.
    Creates a new branch from the latest dev branch after cleaning the workspace.
    """
    repo = git.Repo(WORKSPACE_PATH)
    repo.git.fetch('origin')
    repo.git.reset('--hard')
    repo.git.clean('-fd')
    repo.git.checkout('dev')
    repo.git.pull('origin', 'dev')
    new_branch = repo.create_head(branch_name, 'dev')
    new_branch.checkout()
    return f"Successfully created and checked out fresh branch '{branch_name}' from dev"

PROTECTED_BRANCHES = ['dev', 'main', "staging", "test"]

@tool
def commit_and_push_changes(
    message: Annotated[str, "The commit message"]
) -> Annotated[str, "The status message indicating success or error"]:
    """
    Commit and push the changes to the current branch.
    """
    repo = git.Repo(WORKSPACE_PATH)
    repo.git.add('.')
    repo.git.commit('-m', message)
    new_branch = repo.active_branch
    if new_branch.name in PROTECTED_BRANCHES:
        return f"Cannot push to protected branch '{new_branch.name}'"
    repo.git.push('origin', new_branch)
    return f"Successfully committed and pushed changes to branch '{repo.active_branch}'"

@tool
def create_pr(
    title: Annotated[str, "The title of the PR"],
    body: Annotated[str, "The body of the PR"]
) -> Annotated[str, "The status message indicating success or error"]:
    """
    Create a PR for the current branch against the dev branch.
    """
    token = os.getenv('REDGOLD_AI_TOKEN')
    if not token:
        return "Error: REDGOLD_AI_TOKEN environment variable not found"
    
    try:
        repo = git.Repo(WORKSPACE_PATH)
        current_branch = repo.active_branch
        
        if current_branch.name in PROTECTED_BRANCHES:
            return f"Cannot create PR from protected branch '{current_branch.name}'"
        
        # Create GitHub PR
        g = Github(token)
        gh_repo = g.get_repo("redgold-io/redgold")
        
        # Create the pull request
        pr = gh_repo.create_pull(
            title=title,
            body=body,
            head=current_branch.name,
            base='dev'
        )
        
        return f"Successfully created PR #{pr.number}: {pr.html_url}"
    
    except Exception as e:
        return f"Error creating PR: {str(e)}"
    


@tool
def get_my_active_pull_requests() -> Annotated[list[str], "The active pull requests for the current user"]:
    """
    Get the active pull requests for the current user, including PR body, status, and conversations.
    Returns a list of formatted strings containing detailed PR information.
    """
    token = os.getenv('REDGOLD_AI_TOKEN')
    if not token:
        return "Error: REDGOLD_AI_TOKEN environment variable not found"
    
    g = Github(token)
    gh_repo = g.get_repo("redgold-io/redgold")
    prs = gh_repo.get_pulls(state='open')
    
    detailed_prs = []
    for pr in prs:
        # Skip PRs not created by redgold-ai
        if pr.user.login != 'redgold-ai':
            continue
            
        # Get PR status checks
        status = pr.get_commits().reversed[0].get_combined_status()
        status_str = f"Status: {status.state}" if status else "Status: No status checks"
        
        # Get PR conversations
        comments = []
        for comment in pr.get_issue_comments():
            comments.append(f"  {comment.user.login}: {comment.body}")
        
        # Format PR details
        pr_details = [
            f"PR #{pr.number}: {pr.title}",
            f"Status: {status_str}",
            f"Created by: {pr.user.login}",
            f"Branch: {pr.head.ref} → {pr.base.ref}",
            "",
            "Description:",
            pr.body if pr.body else "No description provided",
            "",
            "Comments:" if comments else "No comments"
        ]
        pr_details.extend(comments)
        
        detailed_prs.append("\n".join(pr_details))
    
    return detailed_prs if detailed_prs else ["No active pull requests found for redgold-ai"]

@tool
def run_e2e_test() -> Annotated[str, "The output logs of the e2e test"]:
    """
    Run a subprocess of ./bin/e2e.sh which runs the e2e test for the current branch.
    """
    # chdir to current directory workspace
    os.chdir(WORKSPACE_PATH)
    return subprocess.check_output(['./bin/e2e.sh']).decode('utf-8')

@tool
def respond_to_pr_comment(
    pr_number: Annotated[int, "The number of the pull request to respond to"],
    response: Annotated[str, "The response to the comment"]
) -> Annotated[str, "The status message indicating success or error"]:
    """
    Respond to a comment on a pull request.
    """
    token = os.getenv('REDGOLD_AI_TOKEN')
    if not token:
        return "Error: REDGOLD_AI_TOKEN environment variable not found"
    
    g = Github(token)
    gh_repo = g.get_repo("redgold-io/redgold")
    pr = gh_repo.get_pull(pr_number)    
    comment = pr.create_issue_comment(response)
    return f"Successfully responded to PR #{pr_number} with comment ID {comment.id}"


@tool 
def get_pr_failing_test_logs(
    pr_number: Annotated[int, "The number of the pull request to get the failing test logs for"]
) -> Annotated[str, "The actions info"]:
    """
    Get the failing test logs from GitHub Actions for a specific PR.
    Filters out lines that begin with "Compiling ".
    """
    token = os.getenv('REDGOLD_AI_TOKEN')
    if not token:
        return "Error: REDGOLD_AI_TOKEN environment variable not found"
    
    try:
        g = Github(token)
        gh_repo = g.get_repo("redgold-io/redgold")
        pr = gh_repo.get_pull(pr_number)
        
        # Get the latest commit
        latest_commit = list(pr.get_commits())[-1]
        
        # Get workflow runs for this commit
        runs = gh_repo.get_workflow_runs(head_sha=latest_commit.sha)
        
        failing_logs = []
        for run in runs:
            if run.conclusion == "failure":
                # Get jobs for the failed run
                for job in run.jobs():
                    if job.conclusion == "failure":
                        # Get the logs using the GitHub API directly
                        headers = {
                            "Authorization": f"token {token}",
                            "Accept": "application/vnd.github.v3+json"
                        }
                        logs_url = f"https://api.github.com/repos/redgold-io/redgold/actions/jobs/{job.id}/logs"
                        import requests
                        response = requests.get(logs_url, headers=headers)
                        if response.status_code == 200:
                            logs = response.text
                            # Filter out lines starting with "Compiling "
                            filtered_logs = "\n".join([
                                line for line in logs.split("\n")
                                if not line.strip().startswith("Compiling ")
                            ])
                            failing_logs.append(f"Failed Job: {job.name}\n{filtered_logs}")
                        else:
                            failing_logs.append(f"Failed to get logs for job {job.name}: {response.status_code}")
        
        if failing_logs:
            return "\n\n".join(failing_logs)
        return "No failing tests found"
        
    except Exception as e:
        return f"Error getting failing test logs: {str(e)}"


from langchain_community.agent_toolkits.load_tools import load_tools


# BUILTIN_TOOLS = load_tools(
    # [
        # "serpapi"
    # ]
# )

def get_tools():
    TOOLS = [
    active_workspace_redgold_cargo_rust_compile,
    index_and_full_text_repo_search,
    edit_active_workspace_file,
    active_workspace_read_file,
    # active_workspace_create_file,
    active_workspace_get_git_diff,
    get_git_issues,
    get_git_issue_by_number,
    create_fresh_dev_branch,
    commit_and_push_changes,
    create_pr,
    get_my_active_pull_requests,
    run_e2e_test,
    respond_to_pr_comment,
    ]

    from langchain_community.agent_toolkits.load_tools import _BASE_TOOLS, _EXTRA_OPTIONAL_TOOLS, _EXTRA_LLM_TOOLS, _LLM_TOOLS, DANGEROUS_TOOLS
    # rn (
    #         list(_BASE_TOOLS)
    #         + list(_EXTRA_OPTIONAL_TOOLS)
    #         + list(_EXTRA_LLM_TOOLS)
    #         + list(_LLM_TOOLS)
    #         + list(DANGEROUS_TOOLS)
    #     )

    tool_names = []
    # tool_names.extend(list(_BASE_TOOLS.keys()))
    # tool_names.extend(list(_EXTRA_OPTIONAL_TOOLS.keys()))
    # tool_names.extend(list(_EXTRA_LLM_TOOLS.keys()))
    # tool_names.extend(list(_LLM_TOOLS.keys()))
    tool_names.extend(list(DANGEROUS_TOOLS.keys()))
    tool_names = [k for k in tool_names if k != "requests"]

    TOOLS.extend(load_tools(tool_names, allow_dangerous_tools=True))

    # from langchain_community.tools.shell import ShellTool
    # shell_tool = ShellTool()
    # TOOLS.append(shell_tool)


    # now do tavily:
    from langchain_community.tools.tavily_search import TavilySearchResults
    tavily_tool = TavilySearchResults(max_results=5)
    TOOLS.append(tavily_tool)

    # and theo ther tavily thing.
    from langchain_community.tools.tavily_search import TavilyAnswer
    tavily_answer_tool = TavilyAnswer()
    TOOLS.append(tavily_answer_tool)


    from langchain_community.tools.file_management.copy import CopyFileTool
    from langchain_community.tools.file_management.delete import DeleteFileTool
    from langchain_community.tools.file_management.file_search import FileSearchTool
    from langchain_community.tools.file_management.list_dir import ListDirectoryTool
    from langchain_community.tools.file_management.move import MoveFileTool
    from langchain_community.tools.file_management.read import ReadFileTool
    from langchain_community.tools.file_management.write import WriteFileTool

    # Initialize file management tools
    file_management_tools = [
        CopyFileTool(),
        DeleteFileTool(),
        FileSearchTool(),
        ListDirectoryTool(),
        MoveFileTool(),
        ReadFileTool(),
        WriteFileTool()
    ]

    for tool in file_management_tools:
        tool.root_dir = str(WORKSPACE_PATH)

    # Add file management tools to TOOLS list
    TOOLS.extend(file_management_tools)


    # from langchain_community.tools.playwright import (
    #     ClickTool,
    #     CurrentWebPageTool,
    #     ExtractHyperlinksTool,
    #     ExtractTextTool,
    #     GetElementsTool,
    #     NavigateBackTool,
    #     NavigateTool,
    # )
    # from playwright.sync_api import sync_playwright

    # # Initialize the browser
    # playwright = sync_playwright().start()
    # browser = playwright.chromium.launch(headless=True)
    # page = browser.new_page()

    # # Create Playwright tools with the initialized browser
    # playwright_tools = [
    #     ClickTool(sync_browser=browser),
    #     CurrentWebPageTool(sync_browser=browser),
    #     ExtractHyperlinksTool(sync_browser=browser),
    #     ExtractTextTool(sync_browser=browser),
    #     GetElementsTool(sync_browser=browser),
    #     NavigateBackTool(sync_browser=browser),
    #     NavigateTool(sync_browser=browser),
    # ]

    # TOOLS.extend(playwright_tools)


    # from langchain_community.agent_toolkits.github.toolkit import GitHubToolkit
    # from langchain_community.utilities.github import GitHubAPIWrapper

    # # Initialize GitHub API wrapper
    # github = GitHubAPIWrapper(
    #     github_repository="redgold-io/redgold",
    #     github_app_id='1138915',
    #     github_app_private_key='./github-app.pem'
    # )
    # # print("Debug: GitHubAPIWrapper initialized")
    # toolkit = GitHubToolkit.from_github_api_wrapper(github)
    # # print("Debug: GitHubToolkit created")

    # # Get tools and rename them to be compliant
    # github_tools = toolkit.get_tools()
    # name_mapping = {
    #     "Get Issues": "github_get_issues",
    #     "Get Issue": "github_get_issue",
    #     "Comment on Issue": "github_comment_on_issue",
    #     "List open pull requests (PRs)": "github_list_open_prs",
    #     "Get Pull Request": "github_get_pr",
    #     "Overview of files included in PR": "github_get_pr_files",
    #     "Create Pull Request": "github_create_pr",
    #     "List Pull Requests' Files": "github_list_pr_files",
    #     "Create File": "github_create_file",
    #     "Read File": "github_read_file",
    #     "Update File": "github_update_file",
    #     "Delete File": "github_delete_file",
    #     "Overview of existing files in Main branch": "github_list_main_files",
    #     "Overview of files in current working branch": "github_list_working_files",
    #     "List branches in this repository": "github_list_branches",
    #     "Set active branch": "github_set_branch",
    #     "Create a new branch": "github_create_branch",
    #     "Get files from a directory": "github_list_directory",
    #     "Search issues and pull requests": "github_search_issues_prs",
    #     "Search code": "github_search_code",
    #     "Create review request": "github_request_review"
    # }

    # for tool in github_tools:
    #     if tool.name in name_mapping:
    #         tool.name = name_mapping[tool.name]


    # banned_tools = ["github_list_main_files"]
    # github_tools = [t for t in github_tools if t.name not in banned_tools]

    # # print("Debug: Renamed GitHub tools")
    # TOOLS.extend(github_tools)
    # print("Debug: Extended TOOLS with renamed GitHub tools")


    uniques = []
    used_names = []
    for tool in TOOLS:
        if tool.name in used_names:
            pass
#            print(tool.name)
        else:
            used_names.append(tool.name)
            uniques.append(tool)

    return uniques