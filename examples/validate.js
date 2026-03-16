const required = ['DB_HOST', 'API_KEY'];
console.log("🟨 Node.js Environment Validator Started...");
let missing = [];
required.forEach(varName => {
    if (!process.env[varName]) {
        console.log(`❌ Missing: ${varName}`);
        missing.push(varName);
    } else {
        console.log(`✅ Found: ${varName}`);
    }
});
if (missing.length > 0) {
    console.log(`\n⚠️  Error: ${missing.length} variable(s) missing.`);
    process.exit(1);
} else {
    console.log("\n🎉 Success: All variables present!");
    process.exit(0);
}
