import { type Locale, t } from "@/i18n";

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
      window.google ? resolve(window.google) : reject(new Error(t.app.signIn.googleUnavailable));
    script.onerror = () => {
      loading = null;
      reject(new Error(t.app.signIn.cannotLoadGoogle));
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
  language: Locale,
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
    locale: language,
    // Google caps the width at 400 px; filling the panel lines it up with the guest form.
    width: Math.min(400, Math.floor(element.clientWidth)),
  });
}

export function googleSignInSupportedHere(): boolean {
  if (__NATIVE_APP__) return true;
  const { protocol, hostname } = window.location;
  return protocol === "https:" || hostname === "localhost" || hostname === "127.0.0.1";
}

/** Cancelled by the user: not an error worth showing. */
export class GoogleSignInCancelled extends Error {}

let nativeInitializedFor: string | null = null;

/**
 * Android: the system account picker (Credential Manager). The ID token's audience is the Web
 * client ID, so the server verifies it exactly like a web sign-in; Google matches the app by
 * package name and signing certificate against the Android OAuth clients.
 */
export async function signInWithGoogleNatively(webClientId: string): Promise<string> {
  if (!__NATIVE_APP__) throw new Error("native Google sign-in exists only in the app");
  const { SocialLogin } = await import("@capgo/capacitor-social-login");
  if (nativeInitializedFor !== webClientId) {
    // No access token needed: skipping Google's extra authorization step avoids a known hang on
    // the stock Capacitor activity.
    await SocialLogin.initialize({ google: { webClientId, skipAuthOnlyAuthorizationAfterSignIn: true } });
    nativeInitializedFor = webClientId;
  }
  try {
    const response = await SocialLogin.login({ provider: "google", options: {} });
    const result = response.result;
    if (result.responseType !== "online" || !result.idToken)
      throw new Error(t.app.signIn.googleReturnedNoToken);
    return result.idToken;
  } catch (error) {
    if (typeof error === "object" && error !== null && "code" in error && error.code === "USER_CANCELLED") {
      throw new GoogleSignInCancelled("cancelled", { cause: error });
    }
    throw error;
  }
}
