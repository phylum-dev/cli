---
title: Projects
category: 61e72e3a50a88e001a92ee5d
---

Projects allow you to track dependencies for a given software repository (e.g. a Git repository) over time. Every submission *must* be associated with a Phylum project. 

### Creating a new project
To create a new project and associate the working directory with it, simply run: 

```
phylum projects create <project-name>
```

This will create a `.phylum_project` file in the current working directory which should be committed into version control.

### Link an existing project
If you have an existing project you want to associate the current working directory with, you can link it by running: 
```
phylum projects link <project-name>
```
This will also create a `.phylum_project` file in the current working directory and all analysis will be done against this Phylum project.

To view a list of your projects, run: 
```
phylum projects list
```
