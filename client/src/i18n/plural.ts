const russianRules = new Intl.PluralRules("ru");

/** The English form for `count`: "1 win", "2 wins". */
export function pluralEn(count: number, one: string, other: string): string {
  return count === 1 ? one : other;
}

/** The Russian form for `count`: 1 победа, 2 победы, 5 побед (21 победа, 11 побед). */
export function pluralRu(count: number, one: string, few: string, many: string): string {
  switch (russianRules.select(count)) {
    case "one":
      return one;
    case "few":
      return few;
    default:
      return many;
  }
}
