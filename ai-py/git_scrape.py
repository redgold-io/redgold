import requests
import json
from datetime import datetime


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


def format_issue_llm(issue):
    labels = " ".join([f"#{label['name']}" for label in issue['labels']])
    created_at = datetime.strptime(issue['created_at'], "%Y-%m-%dT%H:%M:%SZ").strftime("%Y-%m-%d %H:%M:%S")
    updated_at = datetime.strptime(issue['updated_at'], "%Y-%m-%dT%H:%M:%SZ").strftime("%Y-%m-%d %H:%M:%S")
    created_by = issue['user']['login']
    formatted_issue = (
        f"{issue['state']} - {created_by} - {issue['number']} - {issue['title']} - {labels}\n"
        f"{issue['body']}\n"
        f"created_at: {created_at}, updated_at: {updated_at}\n"
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

if __name__ == "__main__":
    main()