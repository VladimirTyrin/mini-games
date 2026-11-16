interface GoogleIdentityServices {
  accounts: {
    id: {
      initialize(config: { client_id: string; callback: (response: { credential: string }) => void }): void;
      renderButton(element: HTMLElement, options: Record<string, string | number>): void;
    };
  };
}

declare global {
  interface Window {
    google?: GoogleIdentityServices;
  }
}

let loading: Promise<GoogleIdentityServices> | null = null;

function loadScript(): Promise<GoogleIdentityServices> {
  loading ??= new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = "https://accounts.google.com/gsi/client";
    script.async = true;
    script.onload = () =>
      window.google ? resolve(window.google) : reject(new Error("Google sign-in unavailable"));
    script.onerror = () => {
      loading = null;
      reject(new Error("cannot load Google sign-in"));
    };
    document.head.append(script);
  });
  return loading;
}

/**
 * Renders Google's own sign-in button (web). Google only allows registered https origins (and
 * localhost), so on a bare-IP http page the button will not work — the caller explains that.
 */
export async function renderGoogleButton(
  element: HTMLElement,
  clientId: string,
  onIdToken: (idToken: string) => void,
): Promise<void> {
  const google = await loadScript();
  google.accounts.id.initialize({
    client_id: clientId,
    callback: (response) => onIdToken(response.credential),
  });
  google.accounts.id.renderButton(element, {
    theme: "filled_black",
    size: "large",
    shape: "pill",
    text: "signin_with",
  });
}

export function googleSignInSupportedHere(): boolean {
  const { protocol, hostname } = window.location;
  return protocol === "https:" || hostname === "localhost" || hostname === "127.0.0.1";
}
