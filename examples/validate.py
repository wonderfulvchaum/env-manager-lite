import os
import sys

def validate_env():
    print("🐍 Python Environment Validator Started...")
    required_vars = ["DB_HOST", "API_KEY"]
    missing = []

    for var in required_vars:
        if not os.getenv(var):
            missing.append(var)
            print(f"❌ Missing: {var}")
        else:
            print(f"✅ Found: {var}")

    if missing:
        print(f"\n⚠️  Error: {len(missing)} variable(s) missing.")
        sys.exit(1)
    else:
        print("\n🎉 Success: All required variables are present!")
        sys.exit(0)

if __name__ == "__main__":
    validate_env()
