import React from 'react'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { vi } from 'vitest'

// Mock portal to render inline
vi.mock('react-dom', () => ({ createPortal: (el: any) => el }))

// Mock heavy child components used by Workloads
vi.mock('../components/CreatePodDialog', () => ({
  CreatePodDialog: (props: any) => (
    <div data-testid="create-pod-dialog">{props.open ? 'open' : 'closed'}</div>
  ),
}))
vi.mock('../components/LogsDialog', () => ({
  LogsDialog: (props: any) => (
    <div data-testid="logs-dialog">{props.open ? 'open' : 'closed'}</div>
  ),
}))
vi.mock('../components/TerminalView', () => ({
  TerminalView: (props: any) => (
    <div data-testid="terminal-view">{props.isVisible ? 'open' : 'closed'}</div>
  ),
}))
vi.mock('../components/YamlEditor', () => ({
  YamlEditor: (props: any) => (
    <div data-testid="yaml-editor">{props.open ? 'open' : 'closed'}</div>
  ),
}))

import { Workloads } from '../components/Workloads'

describe('Workloads component', () => {
  const initialPods = [
    {
      name: 'frontend-1',
      image: 'fe-image:1.0',
      labels: { app: 'frontend' },
      node: 'node-a',
      status: 'Running',
      cpuUsage: '10%',
      memoryUsage: '20%',
      age: '2m',
      ready: '1/1',
      restarts: 0,
      ip: '10.0.0.1',
    },
    {
      name: 'backend-1',
      image: 'be-image:1.0',
      labels: { app: 'backend' },
      node: 'node-b',
      status: 'Pending',
      cpuUsage: '5%',
      memoryUsage: '10%',
      age: '5m',
      ready: '0/1',
      restarts: 1,
      ip: '10.0.0.2',
    },
  ]

  it('renders pods and filters by search term', async () => {
    const onPodClick = vi.fn()

    function Wrapper() {
      const [pods, setPods] = React.useState(initialPods)
      const [events, setEvents] = React.useState<any[]>([])
      return (
        <Workloads
          pods={pods}
          setPods={setPods}
          recentEvents={events}
          setRecentEvents={setEvents}
          onPodClick={onPodClick}
        />
      )
    }

    render(<Wrapper />)

    // initial count badge
    expect(screen.getByText(/2 Pods/)).toBeInTheDocument()

    // search for frontend
    const input = screen.getByPlaceholderText(/Search workloads.../i)
    fireEvent.change(input, { target: { value: 'frontend' } })

    expect(screen.getByText(/frontend-1/i)).toBeInTheDocument()
    expect(screen.queryByText(/backend-1/i)).not.toBeInTheDocument()
  })

  it('calls onPodClick when pod name is clicked', async () => {
    const onPodClick = vi.fn()

    function Wrapper() {
      const [pods, setPods] = React.useState(initialPods)
      const [events, setEvents] = React.useState<any[]>([])
      return (
        <Workloads
          pods={pods}
          setPods={setPods}
          recentEvents={events}
          setRecentEvents={setEvents}
          onPodClick={onPodClick}
        />
      )
    }

    render(<Wrapper />)

    const podButton = screen.getByText('frontend-1')
    fireEvent.click(podButton)
    expect(onPodClick).toHaveBeenCalledWith('frontend-1')
  })

  it('opens context menu and deletes a pod', async () => {
    const onPodClick = vi.fn()

    function Wrapper() {
      const [pods, setPods] = React.useState(initialPods)
      const [events, setEvents] = React.useState<any[]>([])
      return (
        <Workloads
          pods={pods}
          setPods={setPods}
          recentEvents={events}
          setRecentEvents={setEvents}
          onPodClick={onPodClick}
        />
      )
    }

    render(<Wrapper />)

    // find the table row for frontend-1 and click the trailing menu button
    const podCell = screen.getByText('frontend-1')
    const row = podCell.closest('tr') as HTMLElement
    expect(row).toBeTruthy()
    const buttons = row.querySelectorAll('button')
    const moreBtn = buttons[buttons.length - 1]
    fireEvent.click(moreBtn)

    // menu should render (portal)
    await screen.findByText(/View Logs/i)

    // find delete in the menu and click it
    const menuContainer = screen.getByText(/View Logs/i).closest('div') as HTMLElement
    const menuDeleteBtn = Array.from(menuContainer.querySelectorAll('button')).find(b => b.textContent?.includes('Delete Pod'))
    expect(menuDeleteBtn).toBeTruthy()
    fireEvent.click(menuDeleteBtn!)

    // dialog should appear; click the dialog delete button
    const deleteButtons = screen.getAllByText(/Delete Pod/i)
    const dialogDelete = deleteButtons[deleteButtons.length - 1]
    fireEvent.click(dialogDelete)

    // frontend-1 should be removed
    await waitFor(() => expect(screen.queryByText('frontend-1')).not.toBeInTheDocument())
  })
})
