export const root: string;
export const LANGUAGES: string[];
export function readPushTexts(): Record<string, Record<string, string>>;
export function pushTextOutputs(texts?: Record<string, Record<string, string>>): Record<string, string>;
