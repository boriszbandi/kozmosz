import React from 'react';
import { HelmetProvider, Helmet } from 'react-helmet-async';
import { useLocation } from 'react-router-dom';

const PageTitle = () => {
  const location = useLocation();
  const pageName = location.pathname.substring(1) || 'Kezdőlap';
  const title = `Kozmosz - ${pageName.charAt(0).toUpperCase() + pageName.slice(1)}`;

  return (
    <HelmetProvider>
      <Helmet>
        <title>{title}</title>
      </Helmet>
    </HelmetProvider>
  );
};

export default PageTitle;
