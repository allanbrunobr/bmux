# BMUX Agent Integration

When running inside a BMUX session, you have access to the multi-agent
communication layer. Use these commands to collaborate with other agents.

## Environment Variables

- `BMUX_SESSION` — name of the current session
- `BMUX_SOCKET` — path to the IPC socket

## Publish Task Results

After completing any task, publish your result so other agents can use it:

```bash
bmux context set "task:${DESCRIPTION}:result" "${SUMMARY}" -s $BMUX_SESSION
```

## Read Other Agents' Work

```bash
bmux context dump -s $BMUX_SESSION
bmux context get "task:design:result" -s $BMUX_SESSION
```

## Delegate Subtasks

```bash
bmux task send agent-name "implement the login module" -s $BMUX_SESSION
bmux task send --auto "write tests for auth" -s $BMUX_SESSION
```

## Check Available Agents

```bash
bmux agent list -s $BMUX_SESSION
bmux agent status agent-name -s $BMUX_SESSION
```

## Task Queue

```bash
bmux task list -s $BMUX_SESSION
bmux task status TASK_ID -s $BMUX_SESSION
bmux task cancel TASK_ID -s $BMUX_SESSION
```
