---
title: Label
category: 61e72e3a50a88e001a92ee5d
---

The `-l` label option allows you to specify a label for a given analysis submission under a project. This feature can be particularly useful when leveraging phylum in automation workflows; each label may represent a code branch where dependencies may have changed.  

```sh
phylum analyze -l branch_name package-lock.json
```
<br>
Users can toggle between labeled analysis submissions using the drop-down menu next to the project name on the web application.

![image](https://user-images.githubusercontent.com/34108612/158703571-b28f84ed-74fa-410a-9a51-6de39848de31.png)
