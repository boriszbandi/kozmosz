import React, { useEffect } from 'react';
import { BrowserRouter as Router, Route, Routes, useLocation } from 'react-router-dom';
import HomePage from './Home';
import EventsPage from './Events';
import Navbar from './Navbar';
import PageTitle from './PageTitle';
import './App.css';
import './fadeIn.css';  // Import fade-in CSS
import { createTheme, ThemeProvider } from '@mui/material/styles';

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    background: {
      default: '#121212',
      paper: '#1E1E1E',
    },
    primary: {
      main: '#BB86FC',
    },
    secondary: {
      main: '#03DAC6',
    },
    text: {
      primary: '#FFFFFF',
      secondary: '#B3B3B3',
      disabled: '#757575',
    },
    error: {
      main: '#CF6679',
    },
    divider: '#303030',
  },
  typography: {
    h3: {
      fontFamily: 'Roboto, sans-serif',
      fontWeight: 700,
      fontSize: '2.5rem',
      color: '#BB86FC',
    },
    h6: {
      fontFamily: 'Roboto, sans-serif',
      fontWeight: 400,
      fontSize: '1.25rem',
      color: '#B3B3B3',
    },
    body1: {
      fontFamily: 'Roboto, sans-serif',
      fontSize: '1rem',
      color: '#FFFFFF',
    },
  },
});

function FadeWrapper({ children }) {
  const location = useLocation();  // React Router's hook to detect the route change

  useEffect(() => {
    const page = document.getElementById('fade-wrapper');
    page.classList.remove('fade-in');
    void page.offsetWidth; // Forces a reflow for animation restart
    page.classList.add('fade-in');
  }, [location.pathname]);  // Re-run animation on route change

  return (
    <div id="fade-wrapper" className="fade-in">
      {children}
    </div>
  );
}

function App() {
  return (
    <ThemeProvider theme={darkTheme}>
      <Router>
        <PageTitle />
        <Navbar />
        <FadeWrapper>  {/* Wrap Routes in FadeWrapper */}
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/events" element={<EventsPage />} />
          </Routes>
        </FadeWrapper>
      </Router>
    </ThemeProvider>
  );
}

export default App;
