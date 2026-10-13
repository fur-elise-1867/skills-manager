import { useEffect, useRef, useState } from 'react';
import type { CompanyApi } from '../api';
import type { OperationView } from '../types';
import { errorMessage, isSessionError } from '../types';

export function useCompanyOperation(api: CompanyApi, initial: OperationView, onSucceeded: () => void, onExpired: (message: string) => void) {
  const [operation, setOperation] = useState(initial);
  const [error, setError] = useState('');
  const notified = useRef(false);
  const terminal = ['succeeded', 'failed', 'cancelled', 'recovery_required'].includes(operation.state);

  useEffect(() => {
    if (operation.state === 'awaiting_decision' || terminal) return;
    let active = true;
    const timer = setTimeout(() => api.operationStatus(operation.id).then(next => { if (active) setOperation(next); }).catch(e => { if (active) setError(errorMessage(e)); }), 300);
    return () => { active = false; clearTimeout(timer); };
  }, [api, operation, terminal]);

  useEffect(() => {
    if (operation.state === 'succeeded' && !notified.current) { notified.current = true; onSucceeded(); }
    if (isSessionError(operation.error)) onExpired(errorMessage(operation.error));
  }, [operation, onSucceeded, onExpired]);

  return { operation, setOperation, error, setError, terminal };
}
