import React from 'react';
import { Helmet } from 'react-helmet';
import { useLocation } from 'react-router-dom';

const PageTitle = () => {
  const location = useLocation();
  const pageName = location.pathname.substring(1) || 'Kezdőlap';
  const title = `Kozmosz - ${pageName.charAt(0).toUpperCase() + pageName.slice(1)}`;

  return (
    <Helmet>
      <title>{title}</title>
    </Helmet>
  );
};

export default PageTitle;