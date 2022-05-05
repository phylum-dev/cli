---
title: API Keys
category: 6255e67693d5200013b1fa41
hidden: false
---

API keys provide you with the ability to authenticate with the Phylum API without requiring user credentials. API keys are well suited for CI/CD environments where you may not want to disclose your account information.

The `offline_access` parameter in the `settings.yaml` file contains the API token. The following command can be used to retrieve your token value:  
```sh
phylum auth token
```
The API token can also be set via the environment variable `PHYLUM_API_KEY`. This environment variable will take precedence over the `offline_access` parameter in the `settings.yaml` file.
