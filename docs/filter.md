---
title: Filter
category: 61e72e3a50a88e001a92ee5d
---

The `--filter` option allows you to limit the issues that are displayed based on risk type and impact level.  
<br>
EXAMPLES
* Show only issues with an impact level of at least `high`
```
phylum analyze --verbose --filter=high package-lock.json
```
<br>

* Show issues with an impact level of `critical` in the `author` and `engineering` domains
```
phylum analyze --verbose --filter=crit,auth,eng
```

<br>

`filter` codes:
> Impact level
>     `crit`, `high`, `med`, `low`
>
> Risk type
> `auth`, `eng`, `mal`, `vuln`, `lic`
