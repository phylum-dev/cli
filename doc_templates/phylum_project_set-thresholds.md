---
title: phylum project set-thresholds
category: 6255e67693d5200013b1fa3e
parentDoc: 62757a105ec2660021a19e4d
hidden: false
---

{PH-MARKDOWN}

### Details

Analysis results scoring at or below the defined projects thresholds will cause
the analysis to be marked as failure.

### Examples

```sh
# Interactively set risk domain thresholds for the 'sample' project
$ phylum project set-thresholds sample

# Interactively set risk domain thresholds for the 'sample' project owned by the 'sGroup' group
$ phylum project set-thresholds -g sGroup sample
```
