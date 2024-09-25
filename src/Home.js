// src/HomePage.js
import React from 'react';
import { AppBar, Toolbar, Typography, Button, Container, Box } from '@mui/material';

const HomePage = () => {
  return (
    <div>
      {/* Navbar */}
      <AppBar position="static" sx={{ backgroundColor: 'black' }}>
        <Toolbar>
          <Button color="inherit"><img src="/kozmosz_transparent.svg" alt="Kozmosz Logo" style={{ height: '50px', marginRight: '10px'}} /></Button>
          <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}></Typography>
          <Button color="inherit">Események</Button>
          <Button color="inherit">Közösség</Button>
          <Button color="inherit">Rólunk</Button>
        </Toolbar>
      </AppBar>

      {/* Main Content */}
      <Container maxWidth="md" style={{ textAlign: 'center', marginTop: '50px'}}>
        <Typography variant="h3" gutterBottom>
          Welcome to the Space Enthusiasts Community!
        </Typography>
        <Typography variant="h6" gutterBottom>
          Join us to explore the latest news, events, and discussions about the universe.
        </Typography>

        {/* Call to Action */}
        <Button variant="contained" color="primary" size="large" style={{ marginTop: '30px' }}>
          Join the Community
        </Button>
      </Container>      
    </div>
  );
}

export default HomePage;
