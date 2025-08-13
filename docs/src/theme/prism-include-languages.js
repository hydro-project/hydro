import siteConfig from "@generated/docusaurus.config";
export default function prismIncludeLanguages(PrismObject) {
  const {
    themeConfig: { prism },
  } = siteConfig;
  const { additionalLanguages } = prism;

  const PrismBefore = globalThis.Prism;
  globalThis.Prism = PrismObject;
  additionalLanguages.forEach((lang) => {
    // eslint-disable-next-line global-require, import/no-dynamic-require
    require(`prismjs/components/prism-${lang}`);
  });
  Prism.languages["rust"] = {
    nondet: /nondet!/,
    "nondet-ident": /_?nondet[a-zA-Z_]*/,
    ...Prism.languages.rust,
  };
  Prism.languages["rust,ignore"] = Prism.languages.rust;
  Prism.languages["rust,no_run"] = Prism.languages.rust;
  Prism.languages["compile_fail"] = Prism.languages.rust;

  const origTokenize = PrismObject.tokenize;
  PrismObject.hooks.add("after-tokenize", function (env) {
    if (
      env.language === "rust" ||
      env.language === "rust,ignore" ||
      env.language === "rust,no_run" ||
      env.language === "compile_fail"
    ) {
      let code = env.code
        .split("\n")
        .filter((line) => !line.startsWith("# "))
        .join("\n");
      env.tokens = origTokenize(code, env.grammar);
    }
  });

  delete globalThis.Prism;
  if (typeof PrismBefore !== "undefined") {
    globalThis.Prism = PrismObject;
  }
}
