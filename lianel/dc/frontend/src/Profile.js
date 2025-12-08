import React, { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import UserDropdown from './UserDropdown';
import './App.css';
import './Profile.css';

function Profile() {
  const [userInfo, setUserInfo] = useState({
    id: '',
    username: '',
    email: '',
    firstName: '',
    lastName: '',
    name: '',
    emailVerified: false
  });
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState(false);
  const [changingPassword, setChangingPassword] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  
  // Edit form state
  const [editForm, setEditForm] = useState({
    firstName: '',
    lastName: '',
    email: ''
  });
  
  // Password change form state
  const [passwordForm, setPasswordForm] = useState({
    currentPassword: '',
    newPassword: '',
    confirmPassword: ''
  });

  useEffect(() => {
    fetchUserInfo();
  }, []);

  const fetchUserInfo = async () => {
    try {
      setLoading(true);
      setError('');
      const response = await fetch('/api/profile', {
        method: 'GET',
        credentials: 'include'
      });
      
      if (response.ok) {
        const data = await response.json();
        setUserInfo(data);
        setEditForm({
          firstName: data.firstName || '',
          lastName: data.lastName || '',
          email: data.email || ''
        });
      } else {
        const errorData = await response.json().catch(() => ({}));
        setError(errorData.error || 'Failed to load profile');
      }
    } catch (error) {
      console.error('Error fetching profile:', error);
      setError('Failed to load profile. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const handleEdit = () => {
    setEditing(true);
    setError('');
    setSuccess('');
  };

  const handleCancelEdit = () => {
    setEditing(false);
    setEditForm({
      firstName: userInfo.firstName || '',
      lastName: userInfo.lastName || '',
      email: userInfo.email || ''
    });
    setError('');
  };

  const handleSave = async () => {
    try {
      setError('');
      setSuccess('');
      
      const response = await fetch('/api/profile', {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json'
        },
        credentials: 'include',
        body: JSON.stringify(editForm)
      });

      if (response.ok) {
        const data = await response.json();
        setUserInfo(data);
        setEditing(false);
        setSuccess('Profile updated successfully!');
        setTimeout(() => setSuccess(''), 3000);
      } else {
        const errorData = await response.json().catch(() => ({}));
        setError(errorData.error || errorData.details || 'Failed to update profile');
      }
    } catch (error) {
      console.error('Error updating profile:', error);
      setError('Failed to update profile. Please try again.');
    }
  };

  const handleChangePassword = async () => {
    if (passwordForm.newPassword !== passwordForm.confirmPassword) {
      setError('New passwords do not match');
      return;
    }

    if (passwordForm.newPassword.length < 8) {
      setError('Password must be at least 8 characters long');
      return;
    }

    try {
      setError('');
      setSuccess('');
      
      const response = await fetch('/api/profile/change-password', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        credentials: 'include',
        body: JSON.stringify({
          currentPassword: passwordForm.currentPassword,
          newPassword: passwordForm.newPassword
        })
      });

      if (response.ok) {
        setChangingPassword(false);
        setPasswordForm({
          currentPassword: '',
          newPassword: '',
          confirmPassword: ''
        });
        setSuccess('Password changed successfully!');
        setTimeout(() => setSuccess(''), 3000);
      } else {
        const errorData = await response.json().catch(() => ({}));
        setError(errorData.error || 'Failed to change password');
      }
    } catch (error) {
      console.error('Error changing password:', error);
      setError('Failed to change password. Please try again.');
    }
  };

  const handleLogout = () => {
    // RP-Initiated Logout flow:
    // 1. Clear OAuth2-proxy session
    // 2. Redirect to Keycloak logout with redirect_uri
    // 3. Keycloak shows confirmation page, user clicks Logout
    // 4. Keycloak clears session and should redirect back to redirect_uri
    // Note: Keycloak may not redirect automatically - user may need to manually navigate back
    
    const keycloakLogoutUrl = 'https://auth.lianel.se/realms/lianel/protocol/openid-connect/logout' +
      '?client_id=oauth2-proxy' +
      '&redirect_uri=' + encodeURIComponent('https://www.lianel.se');
    
    // Clear OAuth2-proxy session first, then redirect to Keycloak logout
    window.location.href = '/oauth2/sign_out?rd=' + encodeURIComponent(keycloakLogoutUrl);
  };

  const getInitials = () => {
    if (userInfo.firstName && userInfo.lastName) {
      return `${userInfo.firstName[0]}${userInfo.lastName[0]}`.toUpperCase();
    }
    if (userInfo.name) {
      return userInfo.name.split(' ').map(n => n[0]).join('').toUpperCase().substring(0, 2);
    }
    if (userInfo.username) {
      return userInfo.username.substring(0, 2).toUpperCase();
    }
    return 'U';
  };

  if (loading) {
    return (
      <div className="App">
        <div className="container">
          <div className="loading">Loading profile...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="App">
      <div className="container">
        <header className="header">
          <h1 className="logo">
            <div className="logo-icon">LW</div>
            Lianel World
          </h1>
          <div className="header-right">
            <UserDropdown />
          </div>
        </header>
        
        <main className="main">
          <div className="profile-container">
            <div className="profile-card">
              <div className="profile-header">
                <div className="profile-avatar-large">
                  {getInitials()}
                </div>
                <h2 className="profile-title">User Profile</h2>
              </div>

              {error && (
                <div className="profile-message profile-message-error">
                  {error}
                </div>
              )}

              {success && (
                <div className="profile-message profile-message-success">
                  {success}
                </div>
              )}

              <div className="profile-content">
                <div className="profile-field">
                  <label className="profile-label">Username</label>
                  <div className="profile-value">{userInfo.username || 'N/A'}</div>
                  <div className="profile-hint">Username cannot be changed</div>
                </div>

                {editing ? (
                  <>
                    <div className="profile-field">
                      <label className="profile-label">First Name</label>
                      <input
                        type="text"
                        className="profile-input"
                        value={editForm.firstName}
                        onChange={(e) => setEditForm({ ...editForm, firstName: e.target.value })}
                        placeholder="Enter first name"
                      />
                    </div>

                    <div className="profile-field">
                      <label className="profile-label">Last Name</label>
                      <input
                        type="text"
                        className="profile-input"
                        value={editForm.lastName}
                        onChange={(e) => setEditForm({ ...editForm, lastName: e.target.value })}
                        placeholder="Enter last name"
                      />
                    </div>

                    <div className="profile-field">
                      <label className="profile-label">Email</label>
                      <input
                        type="email"
                        className="profile-input"
                        value={editForm.email}
                        onChange={(e) => setEditForm({ ...editForm, email: e.target.value })}
                        placeholder="Enter email address"
                      />
                      {userInfo.emailVerified && (
                        <div className="profile-hint profile-hint-success">✓ Email verified</div>
                      )}
                    </div>
                  </>
                ) : (
                  <>
                    {(userInfo.firstName || userInfo.lastName) && (
                      <div className="profile-field">
                        <label className="profile-label">Full Name</label>
                        <div className="profile-value">
                          {userInfo.firstName && userInfo.lastName
                            ? `${userInfo.firstName} ${userInfo.lastName}`
                            : userInfo.firstName || userInfo.lastName || userInfo.name}
                        </div>
                      </div>
                    )}

                    <div className="profile-field">
                      <label className="profile-label">Email</label>
                      <div className="profile-value">
                        {userInfo.email || 'N/A'}
                        {userInfo.emailVerified && (
                          <span className="profile-verified"> ✓ Verified</span>
                        )}
                      </div>
                    </div>
                  </>
                )}

                {changingPassword && (
                  <div className="profile-password-section">
                    <div className="profile-field">
                      <label className="profile-label">Current Password</label>
                      <input
                        type="password"
                        className="profile-input"
                        value={passwordForm.currentPassword}
                        onChange={(e) => setPasswordForm({ ...passwordForm, currentPassword: e.target.value })}
                        placeholder="Enter current password"
                      />
                    </div>

                    <div className="profile-field">
                      <label className="profile-label">New Password</label>
                      <input
                        type="password"
                        className="profile-input"
                        value={passwordForm.newPassword}
                        onChange={(e) => setPasswordForm({ ...passwordForm, newPassword: e.target.value })}
                        placeholder="Enter new password (min 8 characters)"
                      />
                    </div>

                    <div className="profile-field">
                      <label className="profile-label">Confirm New Password</label>
                      <input
                        type="password"
                        className="profile-input"
                        value={passwordForm.confirmPassword}
                        onChange={(e) => setPasswordForm({ ...passwordForm, confirmPassword: e.target.value })}
                        placeholder="Confirm new password"
                      />
                    </div>
                  </div>
                )}

                {userInfo.id && (
                  <div className="profile-field">
                    <label className="profile-label">User ID</label>
                    <div className="profile-value profile-value-small">{userInfo.id}</div>
                  </div>
                )}
              </div>

              <div className="profile-actions">
                {editing ? (
                  <>
                    <button onClick={handleCancelEdit} className="profile-button profile-button-secondary">
                      Cancel
                    </button>
                    <button onClick={handleSave} className="profile-button profile-button-primary">
                      Save Changes
                    </button>
                  </>
                ) : changingPassword ? (
                  <>
                    <button 
                      onClick={() => { setChangingPassword(false); setPasswordForm({ currentPassword: '', newPassword: '', confirmPassword: '' }); setError(''); }} 
                      className="profile-button profile-button-secondary"
                    >
                      Cancel
                    </button>
                    <button onClick={handleChangePassword} className="profile-button profile-button-primary">
                      Change Password
                    </button>
                  </>
                ) : (
                  <>
                    <Link to="/" className="profile-button profile-button-secondary">
                      ← Back to Home
                    </Link>
                    <button onClick={handleEdit} className="profile-button profile-button-primary">
                      Edit Profile
                    </button>
                    <button 
                      onClick={() => { setChangingPassword(true); setEditing(false); setError(''); }} 
                      className="profile-button profile-button-tertiary"
                    >
                      Change Password
                    </button>
                    <button onClick={handleLogout} className="profile-button profile-button-danger">
                      Logout
                    </button>
                  </>
                )}
              </div>
            </div>
          </div>
        </main>
        
        <footer className="footer">
          <p>&copy; 2025 Lianel World. All rights reserved.</p>
        </footer>
      </div>
    </div>
  );
}

export default Profile;
