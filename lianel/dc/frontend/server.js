const express = require('express');
const path = require('path');
const https = require('https');
const http = require('http');
const app = express();

// Middleware to parse JSON
app.use(express.json());

// Helper function to make HTTP/HTTPS requests
function makeRequest(url, options = {}) {
  return new Promise((resolve, reject) => {
    const urlObj = new URL(url);
    const client = urlObj.protocol === 'https:' ? https : http;
    
    const reqOptions = {
      hostname: urlObj.hostname,
      port: urlObj.port || (urlObj.protocol === 'https:' ? 443 : 80),
      path: urlObj.pathname + urlObj.search,
      method: options.method || 'GET',
      headers: options.headers || {}
    };

    const req = client.request(reqOptions, (res) => {
      let data = '';
      res.on('data', (chunk) => { data += chunk; });
      res.on('end', () => {
        try {
          resolve({ status: res.statusCode, data: JSON.parse(data), headers: res.headers });
        } catch (e) {
          resolve({ status: res.statusCode, data: data, headers: res.headers });
        }
      });
    });

    req.on('error', reject);
    if (options.body) {
      req.write(options.body);
    }
    req.end();
  });
}

// Legacy API endpoint - redirects to profile service
app.get('/api/user-info', async (req, res) => {
  // This endpoint is kept for backward compatibility
  // New frontend should use /api/profile from profile service
  const userId = req.headers['x-user'] || req.headers['x-auth-request-user'] || '';
  const email = req.headers['x-email'] || req.headers['x-auth-request-email'] || '';
  const accessToken = req.headers['x-auth-request-access-token'] || req.headers['x-forwarded-access-token'] || '';

  let userInfo = {
    username: userId,
    email: email,
    name: userId,
    preferredUsername: userId,
    sub: userId
  };

  // Try to get user details from Keycloak userinfo endpoint if we have an access token
  if (accessToken) {
    try {
      const userinfoUrl = 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/userinfo';
      const userinfoResponse = await makeRequest(userinfoUrl, {
        method: 'GET',
        headers: {
          'Authorization': `Bearer ${accessToken}`,
          'Content-Type': 'application/json'
        }
      });

      if (userinfoResponse.status === 200 && userinfoResponse.data) {
        const keycloakUser = userinfoResponse.data;
        userInfo.username = keycloakUser.preferred_username || keycloakUser.username || userId;
        userInfo.preferredUsername = keycloakUser.preferred_username || keycloakUser.username || userId;
        userInfo.email = keycloakUser.email || email;
        userInfo.name = keycloakUser.name || 
                       (keycloakUser.given_name && keycloakUser.family_name 
                         ? `${keycloakUser.given_name} ${keycloakUser.family_name}` 
                         : keycloakUser.preferred_username || keycloakUser.username || userId);
        userInfo.sub = keycloakUser.sub || userId;
        userInfo.firstName = keycloakUser.given_name;
        userInfo.lastName = keycloakUser.family_name;
      }
    } catch (error) {
      console.error('Error fetching userinfo from Keycloak:', error.message);
      // Fall back to header values
    }
  }

  // Fallback: If we still have UUID as username, try to extract from email
  if (userInfo.username && userInfo.username.includes('-') && userInfo.username.length > 30) {
    // Likely a UUID, try to use email prefix or keep as is
    if (userInfo.email) {
      const emailPrefix = userInfo.email.split('@')[0];
      if (emailPrefix && emailPrefix !== userInfo.username) {
        userInfo.username = emailPrefix;
        userInfo.preferredUsername = emailPrefix;
        if (!userInfo.name || userInfo.name === userInfo.sub) {
          userInfo.name = emailPrefix;
        }
      }
    }
  }

  // Final fallback: use email prefix if name is still UUID
  if (userInfo.email && (!userInfo.name || userInfo.name === userInfo.sub || userInfo.name.includes('-'))) {
    const emailPrefix = userInfo.email.split('@')[0];
    if (emailPrefix && emailPrefix.length < 30) {
      userInfo.name = emailPrefix;
    }
  }

  res.json(userInfo);
});

// Serve static files from the React app build directory
app.use(express.static(path.join(__dirname, 'build')));

// Handle React routing - return all requests to React app
app.get('*', (req, res) => {
  res.sendFile(path.join(__dirname, 'build', 'index.html'));
});

const PORT = process.env.PORT || 80;
app.listen(PORT, '0.0.0.0', () => {
  console.log(`Server is running on port ${PORT}`);
});

