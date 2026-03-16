export default async function(ctx) {
  console.log("Executing stored procedure");

  // Use the database API
  ctx.db.put("users", { name: "Alice", email: "alice@example.com" });
  const user = ctx.db.get("users", "1");
  console.log("Found user:", JSON.stringify(user));

  const results = ctx.db.query("users", { name: "Alice" });
  return { processed: results.length, status: "ok" };
}
