#!/bin/bash
set -e

BASE_URL="http://localhost:3000"
PASS=0
FAIL=0

check_response() {
    local test_name=$1
    local expected_status=$2
    local expected_body=$3
    local actual_status=$4
    local actual_body=$5

    if [ "$actual_status" -eq "$expected_status" ] && echo "$actual_body" | grep -q "$expected_body"; then
        echo "✓ PASS: $test_name"
        ((PASS++))
    else
        echo "✗ FAIL: $test_name"
        echo "  Expected status: $expected_status, got: $actual_status"
        echo "  Expected body contains: $expected_body"
        echo "  Actual body: $actual_body"
        ((FAIL++))
    fi
}

echo "Starting integration tests..."
echo ""

echo "Test 1: GET /api/user/1 (path parameter)"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/user/1")
BODY=$(echo "$RESPONSE" | sed '$d')
STATUS=$(echo "$RESPONSE" | tail -n1)
check_response "GET /api/user/1" 200 "zhangsan@example.com" "$STATUS" "$BODY"

echo ""
echo "Test 2: GET /api/user/123 (different id)"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/user/123")
BODY=$(echo "$RESPONSE" | sed '$d')
STATUS=$(echo "$RESPONSE" | tail -n1)
check_response "GET /api/user/123" 200 "张三" "$STATUS" "$BODY"

echo ""
echo "Test 3: POST /api/login with correct body"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"123456"}')
BODY=$(echo "$RESPONSE" | sed '$d')
STATUS=$(echo "$RESPONSE" | tail -n1)
check_response "POST /api/login (correct body)" 200 "fake-jwt-token" "$STATUS" "$BODY"

echo ""
echo "Test 4: POST /api/login without body (should get 401)"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/login")
BODY=$(echo "$RESPONSE" | sed '$d')
STATUS=$(echo "$RESPONSE" | tail -n1)
check_response "POST /api/login (no body)" 401 "用户名或密码错误" "$STATUS" "$BODY"

echo ""
echo "Test 5: POST /api/login with wrong body"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"wrong","password":"wrong"}')
BODY=$(echo "$RESPONSE" | sed '$d')
STATUS=$(echo "$RESPONSE" | tail -n1)
check_response "POST /api/login (wrong body)" 401 "用户名或密码错误" "$STATUS" "$BODY"

echo ""
echo "Test 6: GET /api/products/123 (regex path)"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/products/123")
BODY=$(echo "$RESPONSE" | sed '$d')
STATUS=$(echo "$RESPONSE" | tail -n1)
check_response "GET /api/products/123" 200 "商品名称" "$STATUS" "$BODY"

echo ""
echo "Test 7: GET /unknown (404)"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/unknown")
BODY=$(echo "$RESPONSE" | sed '$d')
STATUS=$(echo "$RESPONSE" | tail -n1)
check_response "GET /unknown" 404 "no matching spec" "$STATUS" "$BODY"

echo ""
echo "Test 8: DELETE /api/user/1 (wrong method)"
RESPONSE=$(curl -s -w "\n%{http_code}" -X DELETE "$BASE_URL/api/user/1")
BODY=$(echo "$RESPONSE" | sed '$d')
STATUS=$(echo "$RESPONSE" | tail -n1)
check_response "DELETE /api/user/1" 404 "no matching spec" "$STATUS" "$BODY"

echo ""
echo "=========================================="
echo "Results: $PASS passed, $FAIL failed"
echo "=========================================="

if [ $FAIL -gt 0 ]; then
    exit 1
fi
exit 0