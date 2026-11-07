# ClickUp

The tasks of a ClickUp list in Escouade's Kanban, its statuses as columns. You link a list to a project, import its open tasks as tickets, and each task follows its ticket: when the ticket changes column, the task takes the chosen status, and Escouade can leave a comment on it.

## Who it is for

For you if your team tracks its work in ClickUp and you want to hand those tasks to agents without copying them.

## What it needs

- A ClickUp account and its personal API key (it starts with `pk_`), made in ClickUp's settings.
- Nothing to install on your machine.

## What it can do

- Reach `api.clickup.com`, and no other host.
- Receive from you your account's API key, which Escouade keeps in the system keychain.
- Read your workspaces, their spaces, folders and lists, a list's statuses and its open tasks; change a task's status; comment on a task.
- Nothing else: it runs no program, reads no file and receives nothing of what your agents do, apart from the text of the comments Escouade asks it to post.

## Learn more

Its documentation has five tabs: **Install** (connect it step by step), **Use** (what it does, screen by screen), **Permissions** (what it reads, writes and sends), **Troubleshooting** (its messages) and **Develop** (for whoever wants to read or change its code).
