with open("src/main.rs", "r") as f:
    lines = f.readlines()

# Fix first line
if lines[0].startswith("jet//!"):
    lines[0] = "// ! Brain Server v6.1 - Working Version\\n"

with open("src/main.rs", "w") as f:
    f.writelines(lines)

print("✅ Fixed!")
