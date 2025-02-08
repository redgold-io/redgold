from pathlib import Path
import subprocess
from typing import Annotated, Optional
import os
import git
from github import Github

from langchain_core.tools import tool

from es_search import full_text_repo_search
from file_ux.create import create_file
from file_ux.edit_files import edit_file
from file_ux.file_viewer import read_file
from file_ux.git_diffs import get_git_diff
from git_scrape import redgold_issue_by_number, redgold_issues_brief
from tools.commands import redgold_cargo_rust_compile


@tool
def active_workspace_redgold_cargo_rust_compile() -> list[str]:
    """
    Compile the current Redgold Rust project stored in ~/ai/redgold.
    :return:
    """
    return redgold_cargo_rust_compile()


@tool
def index_and_full_text_repo_search(
        query: Annotated[str, "Text to search for"]
) -> Annotated[list[str], "List of results"]:
    """
    Search the entire Redgold repository for the given query. Rebuilds an update to date
    index each time
    :param query: The query to search for.
    :return: A list of results.
    """
    return full_text_repo_search({"query": query})


@tool
def edit_active_workspace_file(
    filename: Annotated[str, "The relative path of the file to edit within the current repository"],
    starting_line: Annotated[Optional[str], "The (inclusive) 1-indexed starting line number to begin replacing. If None, starts at beginning"],
    ending_line: Annotated[Optional[str], "The (inclusive) 1-indexed ending line number to stop replacing. If None, goes to end of file"],
    replacement_lines: Annotated[Optional[list[str]], "The new lines to insert. If None, defaults to empty list"] = None
) -> Annotated[str, "Status message indicating success or error"]:
    """
    Edit a file in the current workspace by replacing lines between starting_line and ending_line with replacement_lines.
    If the file doesn't exist, it will be created along with any necessary parent directories.
    Line numbers are 1-indexed and inclusive.
    """
    return edit_file(filename, starting_line, ending_line, replacement_lines)


@tool
def active_workspace_read_file(
    filename: Annotated[str, "The relative path of the file to read within the current repository"],
    starting_line: Annotated[Optional[int], "The (inclusive) 1-indexed starting line number to begin reading. If None, starts at beginning"],
    ending_line: Annotated[Optional[int], "The (inclusive) 1-indexed ending line number to stop reading. If None, goes to end of file"]
) -> Annotated[list[str], "The content of the file"]:
    """
    Read a file in the current workspace by returning the lines between starting_line and ending_line.
    Line numbers are 1-indexed and inclusive.
    """
    return read_file(filename, starting_line, ending_line)


@tool
def active_workspace_create_file(
    repository_relative_path: Annotated[str, "The relative path of the file to create within the current repository"],
    file_text_to_insert_to_new_file: Annotated[str, "The content of the file"],
    include_as_rust_pub_mod: Annotated[bool, "Whether to include the file as a Rust pub mod"] = True
) -> Annotated[str, "Status message indicating success or error"]:
    """
    Create a new file in the current workspace with the given content.
    If the file already exists, it will be overwritten.
    """
    rel_start = Path.home().joinpath("ai/redgold")
    rel: str = repository_relative_path
    rel = str(rel_start.joinpath(rel))
    # Get parent directory path and create all necessary directories
    parent_dir = Path(rel).parent
    parent_dir.mkdir(parents=True, exist_ok=True)

    with open(rel, "w") as f:
        f.write(file_text_to_insert_to_new_file)
        print(f"File created at {rel}")
    if include_as_rust_pub_mod:
        # first check if libs.rs exists in the same parent directory
        # if not, check for mod.rs
        rel_p = Path(rel)
        fnm = rel_p.name
        cur = rel_p.parent
        lib = cur.joinpath("lib.rs")
        mod = cur.joinpath("mod.rs")
        export_str = f"\npub mod {fnm.split('.')[0]};\n"
        if lib.exists():
            with open(lib, "a") as f:
                f.write(export_str)
        elif mod.exists():
            with open(mod, "a") as f:
                f.write(export_str)
        else:
            print("No lib.rs or mod.rs found in the parent directory, not adding the export line")
    return "Success"
        


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
    Uses REDGOLD_AI_TOKEN environment variable for authentication.
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
    
    return detailed_prs if detailed_prs else ["No active pull requests found"]

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


TOOLS = [
    active_workspace_redgold_cargo_rust_compile,
    index_and_full_text_repo_search,
    edit_active_workspace_file,
    active_workspace_read_file,
    active_workspace_create_file,
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

