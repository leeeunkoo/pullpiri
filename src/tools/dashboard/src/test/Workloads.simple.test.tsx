import React from 'react'
import { render, screen, fireEvent } from '@testing-library/react'

function SimpleWorkloads({ pods }: { pods: any[] }) {
  const [search, setSearch] = React.useState('')
  const filtered = pods.filter(p => p.name.toLowerCase().includes(search.toLowerCase()))
  return (
    <div>
      <h1>PULLPIRI Workloads</h1>
      <div><span>{filtered.length} Pods</span></div>
      <input placeholder="Search workloads..." value={search} onChange={(e) => setSearch(e.target.value)} />
      <ul>
        {filtered.map(p => <li key={p.name}>{p.name}</li>)}
      </ul>
    </div>
  )
}

describe('SimpleWorkloads', () => {
  it('filters pods by search term', () => {
    const pods = [
      { name: 'frontend-1' },
      { name: 'backend-1' }
    ]
    render(<SimpleWorkloads pods={pods} />)

    expect(screen.getByText(/2 Pods/i)).toBeInTheDocument()
    const input = screen.getByPlaceholderText(/Search workloads/i)
    fireEvent.change(input, { target: { value: 'frontend' } })
    expect(screen.getByText(/frontend-1/i)).toBeInTheDocument()
    expect(screen.queryByText(/backend-1/i)).not.toBeInTheDocument()
  })
})
