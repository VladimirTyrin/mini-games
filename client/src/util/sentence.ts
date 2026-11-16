/** "not a pair: …" → "Not a pair: …": the rules' refusals as a sentence on screen. */
export function asSentence(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}
