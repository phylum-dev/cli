---
title: API Keys
category: 61e72e3a50a88e001a92ee5d
---

API keys provide you with the ability to authenticate with the Phylum API without requiring user credentials. API keys are well suited for CI/CD environments where you may not want to disclose your account information.

The `offline_access` parameter in the `settings.yaml` file contains the API token. The following command can be used to retrieve your token value:  
```sh
grep "offline_access" $HOME/.phylum/settings.yaml | sed 's/  offline_access: //'
```
