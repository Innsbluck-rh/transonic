export function flushPromises() {
  return new Promise<void>((resolve) => setTimeout(resolve, 0));
}

export async function waitFor(assertion: () => void) {
  let lastError: unknown;
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      assertion();
      return;
    } catch (error) {
      lastError = error;
      await flushPromises();
    }
  }

  throw lastError;
}
