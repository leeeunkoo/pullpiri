import React from 'react'
import { render, screen, fireEvent, cleanup } from '@testing-library/react'
import { vi } from 'vitest'

// Mock internal ui primitives before importing Header
vi.mock('../components/ui/badge', () => ({ Badge: ({ children }: any) => <span>{children}</span> }))
vi.mock('../components/ui/button', () => ({ Button: ({ children, ...props }: any) => <button {...props}>{children}</button> }))
vi.mock('../components/ui/switch', () => ({ Switch: ({ checked, onCheckedChange }: any) => <input data-testid="switch" type="checkbox" checked={checked} onChange={(e)=> onCheckedChange && onCheckedChange(e.target.checked)} /> }))
vi.mock('../components/ui/input', () => ({ Input: (props: any) => <input {...props} /> }))

// Mock theme hook
const toggleTheme = vi.fn()
vi.mock('../components/ThemeProvider', () => ({ useTheme: () => ({ theme: 'light', toggleTheme }) }))

// Mock lucide icons used by Header with identifiable testids
vi.mock('lucide-react', () => ({
  RefreshCw: (props: any) => <svg data-testid="icon-refresh" {...props} />,
  User: (props: any) => <svg data-testid="icon-user" {...props} />,
  Search: (props: any) => <svg data-testid="icon-search" {...props} />,
  Command: (props: any) => <svg data-testid="icon-command" {...props} />,
  Sun: (props: any) => <svg data-testid="icon-sun" {...props} />,
  Moon: (props: any) => <svg data-testid="icon-moon" {...props} />,
}))

// Import after mocks
import { Header } from '../components/Header'

afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})

describe('Header', () => {
  it('renders mobile header with pod count and theme toggle', () => {
    render(<Header mobile podCount={5} pods={[]} />)

    expect(screen.getByText(/5 Pods/i)).toBeInTheDocument()

    // find theme toggle icon and click its enclosing button
    const moonIcon = screen.getByTestId('icon-moon')
    const btn = moonIcon.closest('button')
    expect(btn).toBeTruthy()
    if (btn) fireEvent.click(btn)
    expect(toggleTheme).toHaveBeenCalled()
  })

  it('renders full header with search and IP input and demo switch', () => {
    render(<Header compact={false} podCount={3} pods={[]} />)

    expect(screen.getByText(/3 Pods Running/i)).toBeInTheDocument()

    // Search input should be present (placeholder)
    expect(screen.getByPlaceholderText(/Search resources/i)).toBeInTheDocument()

    // IP input should be present
    expect(screen.getByPlaceholderText(/Dashboard IP/i)).toBeInTheDocument()

    // Switch should be present
    expect(screen.getByTestId('switch')).toBeInTheDocument()
  })
})
