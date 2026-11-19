import { computed } from "vue";

import { locale } from "@/i18n";

/** The privacy policy page in the interface's language. */
export const privacyPolicyUrl = computed(() => (locale.value === "ru" ? "privacy-ru.html" : "privacy.html"));
