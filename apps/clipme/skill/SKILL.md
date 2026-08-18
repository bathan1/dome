---
name: clipme
description: Copy generated text or shell commands to the user's Windows clipboard with the installed clipme binary. Use when the user says copy, clip, clipboard, send or write to clipme, or asks to put generated content on the clipboard.
---

# Use ClipMe

- Invoke `clipme` through the shell instead of merely showing the user a command that calls it.
- Pass a one-line payload as one safely quoted argument.
- For exact or multiline text, pipe the exact UTF-8 bytes to standard input with `printf '%s'`; do not use `echo`.
- Copy only the requested payload. Exclude Markdown fences, explanations, and trailing newlines unless requested.
- Treat copying a shell command and executing it as separate actions. Copying never authorizes execution.
- After a successful copy, respond with a brief confirmation.
- Do not copy credentials, tokens, private keys, or other secrets unless the user explicitly requests it.

Examples:

```sh
clipme 'git status --short'
printf '%s' 'line one
line two' | clipme
```
