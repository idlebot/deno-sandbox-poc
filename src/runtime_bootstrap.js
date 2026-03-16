globalThis.console = {
  log: (...args) => Deno.core.ops.op_sandbox_log(args.map(String).join(" ")),
  warn: (...args) => Deno.core.ops.op_sandbox_log("[WARN] " + args.map(String).join(" ")),
  error: (...args) => Deno.core.ops.op_sandbox_log("[ERROR] " + args.map(String).join(" ")),
};
