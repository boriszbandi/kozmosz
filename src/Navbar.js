import React from 'react';
import { AppBar, Toolbar, Typography, Button } from '@mui/material';
import { NavLink, useLocation } from 'react-router-dom';

const Navbar = () => {
  const location = useLocation();

  return (
    <AppBar position="sticky" sx={{ backgroundColor: 'black' }}>
      <Toolbar>
        <Button component={NavLink} to="/" color="inherit" sx={{ marginRight: 2 }}>
          <img src="/kozmosz_transparent.svg" alt="Kozmosz Logo" style={{ height: '50px', marginRight: '10px' }} />
        </Button>
        <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}></Typography>

        <Button
          component={NavLink}
          to="/events"
          color="inherit"
          sx={{ color: location.pathname === '/events' ? '#BB86FC' : '#FFFFFF' }}
        >
          Események
        </Button>

        <Button
          component={NavLink}
          to="/community"
          color="inherit"
          sx={{ color: location.pathname === '/community' ? '#BB86FC' : '#FFFFFF' }}
        >
          Közösség
        </Button>

        <Button
          component={NavLink}
          to="/about"
          color="inherit"
          sx={{ color: location.pathname === '/about' ? '#BB86FC' : '#FFFFFF' }}
        >
          Rólunk
        </Button>
      </Toolbar>
    </AppBar>
  );
};

export default Navbar;
