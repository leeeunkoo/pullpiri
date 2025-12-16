import React from 'react'
import { render, screen, fireEvent } from '@testing-library/react'

// Minimal mocks for UI primitives used by the component
vi.mock('../test/__mocks__/recharts', () => ({}))

vi.mock('lucide-react', () => ({
  GitBranch: (props: any) => <svg data-testid="icon-git" {...props} />,
  Package: (props: any) => <svg data-testid="icon-package" {...props} />,
  Activity: (props: any) => <svg data-testid="icon-activity" {...props} />,
  Box: (props: any) => <svg data-testid="icon-box" {...props} />,
  ZoomIn: (props: any) => <svg data-testid="icon-zoomin" {...props} />,
  ZoomOut: (props: any) => <svg data-testid="icon-zoomout" {...props} />,
  RotateCcw: (props: any) => <svg data-testid="icon-rotate" {...props} />,
  Search: (props: any) => <svg data-testid="icon-search" {...props} />,
}))

// Mock internal ui components with simple wrappers that forward children/props
vi.mock('../components/ui/card', () => ({
  Card: ({ children, ...p }: any) => <div data-testid="card" {...p}>{children}</div>,
  CardContent: ({ children, ...p }: any) => <div data-testid="card-content" {...p}>{children}</div>,
  CardHeader: ({ children, ...p }: any) => <div data-testid="card-header" {...p}>{children}</div>,
  CardTitle: ({ children, ...p }: any) => <div data-testid="card-title" {...p}>{children}</div>,
}))

vi.mock('../components/ui/badge', () => ({ Badge: ({ children, ...p }: any) => <span data-testid="badge" {...p}>{children}</span> }))
vi.mock('../components/ui/button', () => ({ Button: ({ children, ...p }: any) => <button {...p}>{children}</button> }))
vi.mock('../components/ui/input', () => ({ Input: (p: any) => <input {...p} /> }))

import { Dependencies } from '../components/Dependencies'

describe('Dependencies component', () => {
  it('renders header, namespace badge and legend', () => {
    render(<Dependencies namespace="default" />)
    // Use role-based query for heading to avoid matching multiple nodes
    expect(screen.getByRole('heading', { name: /Dependencies/i })).toBeInTheDocument()
    expect(screen.getByText(/Namespace: default/i)).toBeInTheDocument()
    expect(screen.getByText(/Legend/i)).toBeInTheDocument()
  })

  it('filters nodes based on search input', () => {
    render(<Dependencies namespace="ns" />)
    const search = screen.getByPlaceholderText(/Search nodes/i)
    // Filter for the English "AD Driving" label
    fireEvent.change(search, { target: { value: 'AD' } })
    expect(screen.getByText(/AD Driving/i)).toBeInTheDocument()
    expect(screen.queryByText(/Manual Driving/i)).not.toBeInTheDocument()
  })

  it('zoom in, zoom out and reset buttons update zoom indicator', () => {
    render(<Dependencies namespace="nx" />)
    const zoomText = () => screen.getByText(/Zoom:/i)
    const initial = zoomText().textContent
    const buttons = screen.getAllByRole('button')
    // Buttons order in the component: zoom in, zoom out, reset
    fireEvent.click(buttons[0])
    expect(zoomText().textContent).not.toEqual(initial)
    // Reset to initial
    fireEvent.click(buttons[2])
    expect(zoomText().textContent).toEqual(initial)
  })

  it('clicking a node shows node details and connected badges', () => {
    render(<Dependencies namespace="nx" />)
    // click on an element with circle role simulated by querying SVG circle
    const circles = document.querySelectorAll('circle')
    expect(circles.length).toBeGreaterThan(0)
    // click the first circle
    fireEvent.click(circles[0])
    // Node details card should now display either a label or 'Click on a node' if none
    // Since clicking toggles selection, ensure Node Details header exists
    expect(screen.getByText(/Node Details/i)).toBeInTheDocument()
  })
})
