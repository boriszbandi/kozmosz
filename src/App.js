import HomePage from './Home';
import './App.css';
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
      color: '#BB86FC', // Custom color for h3
    },
    h6: {
      fontFamily: 'Roboto, sans-serif',
      fontWeight: 400,
      fontSize: '1.25rem',
      color: '#B3B3B3', // Custom color for h6
    },
    body1: {
      fontFamily: 'Roboto, sans-serif',
      fontSize: '1rem',
      color: '#FFFFFF', // Default body text color
    },
  },
});


function App() {
  return (
    <ThemeProvider theme={darkTheme}>
      <HomePage />
    </ThemeProvider>
  );
}

export default App;
