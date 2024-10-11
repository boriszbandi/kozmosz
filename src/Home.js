// src/HomePage.js
import React from 'react';
import { Typography, Button, Container } from '@mui/material';

const HomePage = () => {
  return (
    <div>
      {/* Main Content */}
      <Container maxWidth="md" style={{ textAlign: 'center', marginTop: '50px'}}>
        <Typography variant="h3" gutterBottom>
          Köszöntünk a Kozmosz Közösségben!
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
