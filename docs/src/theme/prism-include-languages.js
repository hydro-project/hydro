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

  // Lean 4 (used by the Hydro Lean docs).
  Prism.languages["lean"] = {
    comment: [
      { pattern: /\/-[\s\S]*?-\//, greedy: true },
      { pattern: /--.*/, greedy: true },
    ],
    string: { pattern: /"(?:\\.|[^"\\])*"/, greedy: true },
    attribute: { pattern: /@\[[^\]]*\]/, alias: "annotation" },
    command: {
      pattern: /#(?:check|print|eval|guard|guard_eval|nondet_census|reduce|synth)\b/,
      alias: "important",
    },
    keyword:
      /\b(?:hydro|def|theorem|lemma|abbrev|structure|inductive|class|instance|where|with|fun|match|let|have|show|from|by|do|if|then|else|for|in|at|import|open|namespace|section|end|variable|universe|mutual|deriving|private|protected|partial|noncomputable|unsafe|macro|syntax|elab|set_option|calc|termination_by|decreasing_by|omit|include|ensures|requires|ghost|prove|tick|fix|via|invariant|rebind|emit|yield|kyield|complete|state|input|obtain|intro|witness|subst|hydro_run)\b/,
    builtin: /\b(?:Prop|Type|Sort|Nat|Bool|List|Array|Option|Multiset|Fin|Unit|String|ByteArray)\b/,
    boolean: /\b(?:true|false|True|False)\b/,
    number: /\b\d+\b/,
    tactic:
      /\b(?:simp|simp_all|rw|rfl|exact|refine|apply|intro|intros|cases|rcases|induction|constructor|omega|decide|trivial|assumption|split|unfold|exfalso|first|try|repeat|congr|funext|subst|generalize|change|rename_i|next|conv|norm_num|aesop|co_simp|co_transfer|co_wf_simp|co_lag|co_lag_le|co_lag_simp|co_residuals|co_knot_sr|co_knot_rr|hydro_rule_apply|hydro_wf_close|hydro_pre_close|hydro_smono_walk|hydro_ghost_have)\b/,
    operator: /:=|=>|->|←|→|↔|∀|∃|λ|⟨|⟩|≤|≥|≠|∧|∨|¬|∈|∉|⊑|<\+:|\|\||&&|[=<>+\-*\/]/,
    punctuation: /[()[\]{},.:;]/,
  };

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
