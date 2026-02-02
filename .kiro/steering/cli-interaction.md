---
inclusion: always
---

# Script Execution Best Practices

**MANAGE OUTPUT TO AVOID CONTEXT OVERLOAD**

- When running AWS commands, always append the "--no-paginate  --no-cli-pager " parameters to prevent paging. If those flags do not work, try to ensure pagination is disabled with aws cli commands somehow.
- Don't launch cdk deployments as background tasks, just run them directly.
- Don't launch cdk integration tests as background tasks, just run them directly.