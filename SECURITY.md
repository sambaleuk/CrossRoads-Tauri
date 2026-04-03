# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do NOT** open a public GitHub issue
2. Email **security@neurogrid.me** with details
3. Include steps to reproduce if possible
4. We will acknowledge within 48 hours

## Security Model

XRoads orchestrates AI coding agents that execute commands on your machine:

- Agents run with the same permissions as your user account
- API keys are stored in browser localStorage (use environment variables in production)
- The `--dangerously-skip-permissions` flag is used for autonomous agent operation
- Safety gates (SIGSTOP/SIGCONT) provide runtime guardrails
- All agent actions are logged and visible in the terminal UI

## Responsible Use

XRoads is a development tool. Use it only on your own code and infrastructure.
