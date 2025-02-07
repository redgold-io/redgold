import requests
from datetime import datetime
import os
import git


def get_github_issues(owner, repo):
    url = f"https://api.github.com/repos/{owner}/{repo}/issues"
    issues = []
    page = 1

    while True:
        response = requests.get(f"{url}?page={page}&per_page=100")
        if response.status_code == 200:
            page_issues = response.json()
            if not page_issues:
                break
            issues.extend(page_issues)
            page += 1
        else:
            print(f"Error: {response.status_code}")
            break

    return issues


def format_issues(issues):
    formatted_issues = []
    for issue in issues:
        formatted_issue = format_issue_llm(issue)
        formatted_issues.append(formatted_issue)
    return formatted_issues


def format_issue_llm(issue, include_body=True):
    labels = " ".join([f"#{label['name']}" for label in issue['labels']])
    created_at = datetime.strptime(issue['created_at'], "%Y-%m-%dT%H:%M:%SZ").strftime("%Y-%m-%d %H:%M:%S")
    updated_at = datetime.strptime(issue['updated_at'], "%Y-%m-%dT%H:%M:%SZ").strftime("%Y-%m-%d %H:%M:%S")
    created_by = issue['user']['login']
    body = f"{issue['body']}\n" if include_body else ""
    formatted_issue = (
        f"{issue['state']} - {created_by} - {issue['number']} - {issue['title']} - {labels}\n"
        f"{body}"
        f"created_at: {created_at}, updated_at: {updated_at}\n"
    )
    return formatted_issue


def format_issue_llm_brief(issue):
    labels = " ".join([f"#{label['name']}" for label in issue['labels']])
    created_by = issue['user']['login']
    formatted_issue = (
        f"{issue['state']} - {created_by} - {issue['number']} - {issue['title']} - {labels}"
    )
    return formatted_issue


def all_issues_one_string():
    issues = redgold_issues()
    formatted_issues = format_issues(issues)
    return '\n'.join(formatted_issues)


def redgold_issues():
    owner = "redgold-io"
    repo = "redgold"
    issues = get_github_issues(owner, repo)
    return issues


def redgold_issues_brief():
    issues = redgold_issues()
    return [format_issue_llm_brief(issue) for issue in issues]

def redgold_issue_by_number(number):
    issues = redgold_issues()
    for issue in issues:
        if issue['number'] == number:
            return format_issue_llm(issue)
    return None

def get_one_ai_dev_issue():
    issues = redgold_issues()
    for issue in issues:
        labels = [label['name'] for label in issue['labels']]
        if 'ai-dev' in labels:
            formatted_issue = format_issue_llm(issue)
            return formatted_issue

def main():
    # print(all_issues_one_string())
    print(get_one_ai_dev_issue())

def create_fresh_dev_branch_workspace(branch_name: str) -> str:
    """
    Creates a fresh development branch in the workspace directory.
    
    Args:
        branch_name: Name of the new branch to create
        
    Returns:
        Status message indicating success or error
    """
    try:
        workspace_dir = os.path.expanduser("~/ai/redgold")
        if not os.path.exists(workspace_dir):
            return f"Error: Workspace directory {workspace_dir} does not exist"
            
        repo = git.Repo(workspace_dir)
        
        # Fetch latest changes
        repo.git.fetch('origin')
        
        # Clean working directory
        repo.git.reset('--hard')
        repo.git.clean('-fd')
        
        # Checkout dev branch and pull latest
        repo.git.checkout('dev')
        repo.git.pull('origin', 'dev')
        
        # Create and checkout new branch
        new_branch = repo.create_head(branch_name, 'dev')
        new_branch.checkout()
        
        return f"Successfully created and checked out fresh branch '{branch_name}' from dev"
        
    except Exception as e:
        return f"Error creating branch: {str(e)}"

if __name__ == "__main__":
    main()