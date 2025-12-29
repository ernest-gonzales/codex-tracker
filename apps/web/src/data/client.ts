type CommandArgs = Record<string, unknown> | undefined;

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function getCsrfToken(): string | undefined {
  const token = (window as Window & { __CODEX_TRACKER_CSRF__?: string })
    .__CODEX_TRACKER_CSRF__;
  return token && token.length > 0 ? token : undefined;
}

export async function invokeCommand<T>(command: string, args?: CommandArgs): Promise<T> {
  if (isTauriRuntime()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(command, args);
  }

  const csrfToken = getCsrfToken();
  const response = await fetch(`/api/${command}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(csrfToken ? { "X-Codex-Token": csrfToken } : {})
    },
    body: JSON.stringify(args ?? {})
  });

  if (!response.ok) {
    let message = `Request failed (${response.status})`;
    try {
      const payload = (await response.json()) as { message?: string };
      if (payload?.message) {
        message = payload.message;
      }
    } catch {
      // ignore JSON parse errors
    }
    throw new Error(message);
  }

  return response.json() as Promise<T>;
}
