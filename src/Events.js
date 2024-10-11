import React from 'react';
import { Typography, Container, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Paper } from '@mui/material';

const EventsPage = () => {
  const events = [
    { name: 'Astronomy Night', date: '2024-10-21', location: 'Budapest' },
    { name: 'Cosmos Meetup', date: '2024-11-05', location: 'Debrecen' },
    { name: 'Space Exploration Talk', date: '2024-12-15', location: 'Pécs' },
  ];

  return (
    <div>
      {/* Main Content */}
      <Container maxWidth="md" style={{ marginTop: '50px' }}>
        <Typography variant="h3" gutterBottom>
          Események
        </Typography>

        {/* Table */}
        <TableContainer component={Paper}>
          <Table>
            <TableHead>
              <TableRow>
                <TableCell>Event Name</TableCell>
                <TableCell>Date</TableCell>
                <TableCell>Location</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {events.map((event, index) => (
                <TableRow key={index}>
                  <TableCell>{event.name}</TableCell>
                  <TableCell>{event.date}</TableCell>
                  <TableCell>{event.location}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </Container>
    </div>
  );
};

export default EventsPage;
