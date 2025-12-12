import React, { useEffect, useState } from 'react';
import { authenticatedFetch } from '../keycloak';

const AdminGuard = ({ children }) => {
  const [loading, setLoading] = useState(true);
  const [allowed, setAllowed] = useState(false);
  const [error, setError] = useState(null);

  useEffect(() => {
    let isMounted = true;
    (async () => {
      try {
        const res = await authenticatedFetch('/api/admin/check');
        if (!res.ok) {
          if (isMounted) {
            setAllowed(false);
            setError(null);
            setLoading(false);
          }
          return;
        }
        const json = await res.json();
        if (isMounted) {
          setAllowed(!!json?.isAdmin);
          setLoading(false);
        }
      } catch (e) {
        if (isMounted) {
          setError('Unable to verify admin access');
          setAllowed(false);
          setLoading(false);
        }
      }
    })();

    return () => { isMounted = false; };
  }, []);

  if (loading) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', padding: '40px' }}>
        Checking permissions…
      </div>
    );
  }

  if (error || !allowed) {
    return (
      <div style={{ maxWidth: 600, margin: '40px auto', textAlign: 'center' }}>
        <h2>403 – Admin Access Required</h2>
        <p>You do not have permission to view this page.</p>
      </div>
    );
  }

  return <>{children}</>;
};

export default AdminGuard;
