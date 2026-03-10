# Verify IBKR live session token (curl)

Get a Keycloak access token (e.g. after logging in at https://www.lianel.se, copy from browser devtools → Application → Local Storage or Network), then run:

```bash
curl -sS "https://www.lianel.se/api/v1/stock-service/ibkr/verify" -H "Authorization: Bearer YOUR_KEYCLOAK_ACCESS_TOKEN" -H "Accept: application/json"
```

- **200** + `{"ok":true,"message":"IBKR authentication verified (LST obtained)"}` = session token verified.
- **401** = invalid/missing Bearer or IBKR consumer config.
- **502** = IBKR API error (e.g. invalid consumer).
